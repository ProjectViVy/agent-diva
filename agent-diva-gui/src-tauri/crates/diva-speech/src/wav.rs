//! C2-4 inbound WAV guard: RIFF, PCM16, mono, 16 kHz, <=120 s, <=8 MiB.
//! Anything else is `invalid_audio` and is rejected BEFORE the provider
//! upload — never sniffed or auto-converted.

use crate::{SpeechCode, SpeechError, SpeechResult};

pub const WAV_MAX_BYTES: usize = 8 * 1024 * 1024;
pub const WAV_MAX_SECONDS: u64 = 120;
pub const WAV_RATE: u32 = 16_000;
pub const WAV_CHANNELS: u16 = 1;
pub const WAV_BITS: u16 = 16;

fn invalid(message: impl Into<String>) -> SpeechError {
    SpeechError::new(SpeechCode::InvalidAudio, message)
}

fn u16le(b: &[u8], off: usize) -> SpeechResult<u16> {
    b.get(off..off + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or_else(|| invalid("truncated WAV header"))
}

fn u32le(b: &[u8], off: usize) -> SpeechResult<u32> {
    b.get(off..off + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| invalid("truncated WAV header"))
}

/// Validate `bytes` as an inbound STT WAV and return the PCM payload
/// length in bytes.
pub fn validate_wav(bytes: &[u8]) -> SpeechResult<u32> {
    if bytes.len() > WAV_MAX_BYTES {
        return Err(invalid("WAV exceeds 8 MiB"));
    }
    if bytes.len() < 12 {
        return Err(invalid("truncated WAV header"));
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(invalid("not a RIFF/WAVE stream"));
    }
    let mut off = 12usize;
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    let mut data_len: Option<usize> = None;
    while off + 8 <= bytes.len() {
        let size = u32le(bytes, off + 4)? as usize;
        let payload = off + 8;
        if payload + size > bytes.len() {
            return Err(invalid("chunk overruns file"));
        }
        match &bytes[off..off + 4] {
            b"fmt " => {
                if size < 16 {
                    return Err(invalid("fmt chunk too small"));
                }
                fmt = Some((
                    u16le(bytes, payload)?,
                    u16le(bytes, payload + 2)?,
                    u32le(bytes, payload + 4)?,
                    u16le(bytes, payload + 14)?,
                ));
            }
            b"data" => {
                data_len = Some(size);
                break;
            }
            _ => {}
        }
        off = payload + size + (size & 1);
    }
    let (format, channels, rate, bits) = fmt.ok_or_else(|| invalid("missing fmt chunk"))?;
    if format != 1 {
        return Err(invalid("WAV must be PCM (format 1)"));
    }
    if channels != WAV_CHANNELS {
        return Err(invalid("WAV must be mono"));
    }
    if rate != WAV_RATE {
        return Err(invalid("WAV must be 16 kHz"));
    }
    if bits != WAV_BITS {
        return Err(invalid("WAV must be 16-bit PCM"));
    }
    let data = data_len.ok_or_else(|| invalid("missing data chunk"))?;
    if data == 0 {
        return Err(invalid("empty audio"));
    }
    if data as u64 > WAV_MAX_SECONDS * WAV_RATE as u64 * 2 {
        return Err(invalid("WAV exceeds 120 s"));
    }
    Ok(data as u32)
}
