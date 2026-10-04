package speech

import (
	"encoding/binary"
	"testing"
)

// makeWAV builds a RIFF/PCM16/mono/16 kHz WAV of the given seconds.
func makeWAV(seconds float64) []byte {
	dataLen := int(seconds * WAVRate * 2)
	data := make([]byte, dataLen)
	var b []byte
	b = append(b, []byte("RIFF")...)
	b = binary.LittleEndian.AppendUint32(b, uint32(36+dataLen))
	b = append(b, []byte("WAVE")...)
	b = append(b, []byte("fmt ")...)
	b = binary.LittleEndian.AppendUint32(b, 16)
	b = binary.LittleEndian.AppendUint16(b, 1)
	b = binary.LittleEndian.AppendUint16(b, WAVChannels)
	b = binary.LittleEndian.AppendUint32(b, WAVRate)
	b = binary.LittleEndian.AppendUint32(b, WAVRate*2)
	b = binary.LittleEndian.AppendUint16(b, 2)
	b = binary.LittleEndian.AppendUint16(b, WAVBits)
	b = append(b, []byte("data")...)
	b = binary.LittleEndian.AppendUint32(b, uint32(dataLen))
	b = append(b, data...)
	return b
}

func codeOf(t *testing.T, err error) SpeechCode {
	t.Helper()
	se, ok := err.(*SpeechError)
	if !ok {
		t.Fatalf("expected *SpeechError, got %T", err)
	}
	return se.Code
}

func TestValidateWAV(t *testing.T) {
	if _, err := ValidateWAV(makeWAV(1)); err != nil {
		t.Fatalf("valid wav rejected: %v", err)
	}
	cases := []struct {
		name string
		mut  func([]byte) []byte
	}{
		{"empty", func(b []byte) []byte { return nil }},
		{"truncated", func(b []byte) []byte { return b[:8] }},
		{"not_riff", func(b []byte) []byte { copy(b[0:4], "NOPE"); return b }},
		{"not_wave", func(b []byte) []byte { copy(b[8:12], "NOPE"); return b }},
		{"wrong_rate", func(b []byte) []byte {
			binary.LittleEndian.PutUint32(b[24:28], 8000)
			return b
		}},
		{"stereo", func(b []byte) []byte {
			binary.LittleEndian.PutUint16(b[22:24], 2)
			return b
		}},
		{"bits8", func(b []byte) []byte {
			binary.LittleEndian.PutUint16(b[34:36], 8)
			return b
		}},
		{"missing_data", func(b []byte) []byte {
			copy(b[36:40], "JUNK")
			return b
		}},
		{"overrun_chunk", func(b []byte) []byte {
			binary.LittleEndian.PutUint32(b[16:20], 0xFFFFFFF0)
			return b
		}},
	}
	for _, tc := range cases {
		if _, err := ValidateWAV(tc.mut(makeWAV(1))); err == nil {
			t.Fatalf("%s: accepted invalid wav", tc.name)
		} else if codeOf(t, err) != CodeInvalidAudio {
			t.Fatalf("%s: code %s, want invalid_audio", tc.name, codeOf(t, err))
		}
	}
	if _, err := ValidateWAV(make([]byte, WAVMaxBytes+1)); err == nil {
		t.Fatal("over-8MiB accepted")
	}
	if _, err := ValidateWAV(makeWAV(121)); err == nil {
		t.Fatal("over-120s accepted")
	}
	if n, err := ValidateWAV(makeWAV(120)); err != nil || n != uint32(120*WAVRate*2) {
		t.Fatalf("120s boundary: %v %d", err, n)
	}
}
