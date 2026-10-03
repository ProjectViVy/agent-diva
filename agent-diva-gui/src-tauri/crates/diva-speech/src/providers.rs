//! DN-6B closed provider adapters: SiliconFlow STT (multipart WAV) and
//! SiliconFlow/MiniMax TTS (JSON). Request shapes are the DN-0S
//! probe-pinned mappings — raw model IDs, no translation parameter, no
//! retries/failover, no cross-host redirects (the shared client disables
//! redirect following entirely).
//!
//! Outbound URLs: HTTPS only. A plain-http endpoint is accepted ONLY for
//! loopback hosts so the offline fixture server can exercise the lane —
//! controlled by `allow_insecure_loopback`, which the shell never sets.

use crate::config::{MiniMaxTts, SiliconFlowTts, SpeechProvider, SpeechReference, SttPreferences};
use crate::{SpeechCode, SpeechError, SpeechResult};

/// C2-4 / DN-C2 caps.
pub const STT_JSON_MAX_BYTES: usize = 64 * 1024;
pub const STT_TEXT_MAX_CHARS: usize = 16 * 1024;
pub const TTS_TEXT_MAX_CHARS: usize = 4000;
pub const TTS_TEXT_MAX_BYTES: usize = 16 * 1024;
pub const MP3_MAX_BYTES: usize = 16 * 1024 * 1024;
pub const MINIMAX_JSON_MAX_BYTES: usize = 34 * 1024 * 1024;

fn provider_error(message: impl Into<String>) -> SpeechError {
    SpeechError::new(SpeechCode::ProviderError, message)
}

/// Outbound URL rule: no credentials/query/fragment (already enforced on
/// the base_url), and HTTPS unless the host is loopback and the caller
/// explicitly allowed insecure loopback (fixture servers only).
pub fn check_request_url(url: &str, allow_insecure_loopback: bool) -> SpeechResult<()> {
    let Some((scheme, rest)) = url.split_once("://") else {
        return Err(provider_error("endpoint has no scheme"));
    };
    match scheme {
        "https" => {}
        "http" => {
            let authority_end = rest.find('/').unwrap_or(rest.len());
            let host = rest[..authority_end].split(':').next().unwrap_or_default();
            let loopback = matches!(host, "127.0.0.1" | "localhost" | "[::1]");
            if !(allow_insecure_loopback && loopback) {
                return Err(provider_error("provider endpoint must be HTTPS"));
            }
        }
        _ => return Err(provider_error("provider endpoint must be HTTPS")),
    }
    Ok(())
}

/// Read a response body with a hard byte cap; oversize is an error, not
/// a truncation.
async fn read_capped(
    resp: reqwest::Response,
    cap: usize,
    provider: SpeechProvider,
) -> SpeechResult<(Vec<u8>, u16)> {
    use futures_util::StreamExt;
    let status = resp.status().as_u16();
    let mut buf = Vec::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| map_transport(e).provider(provider))?;
        if buf.len() + chunk.len() > cap {
            return Err(provider_error("provider response exceeds byte cap")
                .provider(provider)
                .http_status(status));
        }
        buf.extend_from_slice(&chunk);
    }
    Ok((buf, status))
}

fn map_transport(e: reqwest::Error) -> SpeechError {
    if e.is_timeout() {
        return SpeechError::new(SpeechCode::Timeout, "provider request timed out").retryable(true);
    }
    provider_error(format!("provider transport failed: {e}"))
        .retryable(e.is_connect() || e.is_request())
}

fn mp3_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(b"ID3") || (bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0)
}

/// SiliconFlow STT: POST {base}/v1/audio/transcriptions, multipart
/// file=<wav> + model. Success JSON `{text}`; the response body is
/// capped at 64 KiB and the text at 16 KiB.
pub async fn siliconflow_stt(
    client: &reqwest::Client,
    cfg: &SttPreferences,
    key: &str,
    wav: Vec<u8>,
    allow_insecure_loopback: bool,
) -> SpeechResult<(String, u64)> {
    let url = format!(
        "{}/v1/audio/transcriptions",
        cfg.base_url.trim_end_matches('/')
    );
    check_request_url(&url, allow_insecure_loopback)?;
    let part = reqwest::multipart::Part::bytes(wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| provider_error(format!("multipart mime: {e}")))?;
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", cfg.model.clone());
    let resp = client
        .post(&url)
        .bearer_auth(key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| map_transport(e).provider(SpeechProvider::SiliconFlow))?;
    let (body, status) = read_capped(resp, STT_JSON_MAX_BYTES, SpeechProvider::SiliconFlow).await?;
    if !(200..300).contains(&status) {
        return Err(provider_error("STT request rejected")
            .provider(SpeechProvider::SiliconFlow)
            .http_status(status)
            .retryable(status == 429 || status >= 500));
    }
    let json: serde_json::Value = serde_json::from_slice(&body).map_err(|e| {
        provider_error(format!("malformed STT JSON: {e}"))
            .provider(SpeechProvider::SiliconFlow)
            .http_status(status)
    })?;
    let text = json.get("text").and_then(|t| t.as_str()).ok_or_else(|| {
        provider_error("STT response missing text")
            .provider(SpeechProvider::SiliconFlow)
            .http_status(status)
    })?;
    if text.chars().count() > STT_TEXT_MAX_CHARS {
        return Err(provider_error("STT text exceeds 16 KiB cap")
            .provider(SpeechProvider::SiliconFlow)
            .http_status(status));
    }
    Ok((text.to_string(), body.len() as u64))
}

/// SiliconFlow TTS: POST {base}/v1/audio/speech. The voice union is
/// system (`voice` string) XOR reusable `speech:<name>:<id>` uri XOR
/// inline `references` built from an owned asset lease — the lease must
/// outlive the request.
pub async fn siliconflow_tts(
    client: &reqwest::Client,
    cfg: &SiliconFlowTts,
    key: &str,
    text: &str,
    inline: Option<&crate::assets::AssetLease>,
    allow_insecure_loopback: bool,
) -> SpeechResult<(Vec<u8>, u64)> {
    let url = format!("{}/v1/audio/speech", cfg.base_url.trim_end_matches('/'));
    check_request_url(&url, allow_insecure_loopback)?;
    let mut payload = serde_json::json!({
        "model": cfg.model,
        "input": text,
        "response_format": "mp3",
        "stream": false,
    });
    match &cfg.reference {
        Some(SpeechReference::Inline {
            asset_id,
            transcript,
        }) => {
            let Some(lease) = inline else {
                return Err(SpeechError::new(
                    SpeechCode::AssetNotFound,
                    format!("reference asset {asset_id} not leased"),
                ));
            };
            let b64 =
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &lease.bytes);
            payload["references"] = serde_json::json!([{
                "audio": format!("data:{};base64,{}", lease.mime_type, b64),
                "text": transcript,
            }]);
        }
        Some(SpeechReference::Reusable { voice_id }) => {
            payload["voice"] = serde_json::Value::String(voice_id.clone());
        }
        Some(SpeechReference::System(_)) | None => {
            payload["voice"] = serde_json::Value::String(cfg.voice.clone());
        }
    }
    let resp = client
        .post(&url)
        .bearer_auth(key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| map_transport(e).provider(SpeechProvider::SiliconFlow))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let _ = read_capped(resp, STT_JSON_MAX_BYTES, SpeechProvider::SiliconFlow).await;
        return Err(provider_error("TTS request rejected")
            .provider(SpeechProvider::SiliconFlow)
            .http_status(status.as_u16())
            .retryable(status.as_u16() == 429 || status.is_server_error()));
    }
    let (body, status) = read_capped(resp, MP3_MAX_BYTES, SpeechProvider::SiliconFlow).await?;
    if !mp3_magic(&body) {
        return Err(provider_error("provider body is not MP3")
            .provider(SpeechProvider::SiliconFlow)
            .http_status(status));
    }
    let len = body.len() as u64;
    Ok((body, len))
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    let bytes = text.as_bytes();
    for pair in bytes.chunks_exact(2) {
        let hi = (pair[0] as char).to_digit(16)?;
        let lo = (pair[1] as char).to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
    }
    Some(out)
}

/// MiniMax TTS: POST {base}/v1/t2a_v2. HTTP 200 still requires
/// `base_resp.status_code == 0` and `data.status == 2`; `data.audio` is
/// hex MP3 — malformed hex or oversize decode is a provider_error, never
/// successful audio.
pub async fn minimax_tts(
    client: &reqwest::Client,
    cfg: &MiniMaxTts,
    key: &str,
    text: &str,
    allow_insecure_loopback: bool,
) -> SpeechResult<(Vec<u8>, u64)> {
    let url = format!("{}/v1/t2a_v2", cfg.base_url.trim_end_matches('/'));
    check_request_url(&url, allow_insecure_loopback)?;
    let payload = serde_json::json!({
        "model": cfg.model,
        "text": text,
        "stream": false,
        "voice_setting": {
            "voice_id": cfg.voice_id,
            "speed": cfg.speed,
            "vol": cfg.volume,
        },
        "audio_setting": {
            "sample_rate": 32000,
            "bitrate": 128000,
            "format": "mp3",
            "channel": 1,
        },
    });
    let resp = client
        .post(&url)
        .bearer_auth(key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| map_transport(e).provider(SpeechProvider::MiniMax))?;
    let (body, status) = read_capped(resp, MINIMAX_JSON_MAX_BYTES, SpeechProvider::MiniMax).await?;
    if !(200..300).contains(&status) {
        return Err(provider_error("TTS request rejected")
            .provider(SpeechProvider::MiniMax)
            .http_status(status)
            .retryable(status == 429 || status >= 500));
    }
    let json: serde_json::Value = serde_json::from_slice(&body).map_err(|e| {
        provider_error(format!("malformed MiniMax JSON: {e}"))
            .provider(SpeechProvider::MiniMax)
            .http_status(status)
    })?;
    let code = json
        .pointer("/base_resp/status_code")
        .and_then(|v| v.as_i64());
    if code != Some(0) {
        return Err(provider_error("MiniMax business error")
            .provider(SpeechProvider::MiniMax)
            .http_status(status));
    }
    let data_status = json.pointer("/data/status").and_then(|v| v.as_i64());
    if data_status != Some(2) {
        return Err(provider_error("MiniMax audio not ready")
            .provider(SpeechProvider::MiniMax)
            .http_status(status));
    }
    let hex_audio = json
        .pointer("/data/audio")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            provider_error("MiniMax response missing audio")
                .provider(SpeechProvider::MiniMax)
                .http_status(status)
        })?;
    let mp3 = decode_hex(hex_audio).ok_or_else(|| {
        provider_error("malformed MiniMax audio hex")
            .provider(SpeechProvider::MiniMax)
            .http_status(status)
    })?;
    if mp3.len() > MP3_MAX_BYTES {
        return Err(provider_error("MP3 exceeds 16 MiB cap")
            .provider(SpeechProvider::MiniMax)
            .http_status(status));
    }
    if !mp3_magic(&mp3) {
        return Err(provider_error("provider body is not MP3")
            .provider(SpeechProvider::MiniMax)
            .http_status(status));
    }
    Ok((mp3, body.len() as u64))
}
