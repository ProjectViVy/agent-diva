package speech

import "encoding/binary"

// C2-4 inbound WAV guard: RIFF, PCM16, mono, 16 kHz, <=120 s, <=8 MiB.
// Anything else is invalid_audio and is rejected BEFORE the provider
// upload — never sniffed or auto-converted.

const (
	WAVMaxBytes   = 8 * 1024 * 1024
	WAVMaxSeconds = 120
	WAVRate       = 16000
	WAVChannels   = 1
	WAVBits       = 16
)

func invalidAudio(msg string) *SpeechError { return speechErr(CodeInvalidAudio, msg) }

// ValidateWAV validates bytes as an inbound STT WAV and returns the PCM
// payload length in bytes.
func ValidateWAV(b []byte) (uint32, error) {
	if len(b) > WAVMaxBytes {
		return 0, invalidAudio("WAV exceeds 8 MiB")
	}
	if len(b) < 12 {
		return 0, invalidAudio("truncated WAV header")
	}
	if string(b[0:4]) != "RIFF" || string(b[8:12]) != "WAVE" {
		return 0, invalidAudio("not a RIFF/WAVE stream")
	}
	off := 12
	var haveFmt, haveData bool
	var format, channels, bits uint16
	var rate uint32
	var dataLen int
	for off+8 <= len(b) {
		size := int(binary.LittleEndian.Uint32(b[off+4 : off+8]))
		payload := off + 8
		if payload+size > len(b) {
			return 0, invalidAudio("chunk overruns file")
		}
		switch string(b[off : off+4]) {
		case "fmt ":
			if size < 16 {
				return 0, invalidAudio("fmt chunk too small")
			}
			format = binary.LittleEndian.Uint16(b[payload:])
			channels = binary.LittleEndian.Uint16(b[payload+2:])
			rate = binary.LittleEndian.Uint32(b[payload+4:])
			bits = binary.LittleEndian.Uint16(b[payload+14:])
			haveFmt = true
		case "data":
			dataLen = size
			haveData = true
		}
		if haveData {
			break
		}
		off = payload + size + (size & 1)
	}
	if !haveFmt {
		return 0, invalidAudio("missing fmt chunk")
	}
	if format != 1 {
		return 0, invalidAudio("WAV must be PCM (format 1)")
	}
	if channels != WAVChannels {
		return 0, invalidAudio("WAV must be mono")
	}
	if rate != WAVRate {
		return 0, invalidAudio("WAV must be 16 kHz")
	}
	if bits != WAVBits {
		return 0, invalidAudio("WAV must be 16-bit PCM")
	}
	if !haveData {
		return 0, invalidAudio("missing data chunk")
	}
	if dataLen == 0 {
		return 0, invalidAudio("empty audio")
	}
	if uint64(dataLen) > WAVMaxSeconds*WAVRate*2 {
		return 0, invalidAudio("WAV exceeds 120 s")
	}
	return uint32(dataLen), nil
}
