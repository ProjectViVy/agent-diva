package desktop

import (
	"encoding/binary"

	"github.com/ProjectViVy/agent-diva/internal/speech"
)

// speechValidateWAVFixture builds a short valid WAV through the same
// constraints the validator enforces.
func speechValidateWAVFixture() ([]byte, error) {
	dataLen := 1600
	var b []byte
	b = append(b, []byte("RIFF")...)
	b = binary.LittleEndian.AppendUint32(b, uint32(36+dataLen))
	b = append(b, []byte("WAVE")...)
	b = append(b, []byte("fmt ")...)
	b = binary.LittleEndian.AppendUint32(b, 16)
	b = binary.LittleEndian.AppendUint16(b, 1)
	b = binary.LittleEndian.AppendUint16(b, speech.WAVChannels)
	b = binary.LittleEndian.AppendUint32(b, speech.WAVRate)
	b = binary.LittleEndian.AppendUint32(b, speech.WAVRate*2)
	b = binary.LittleEndian.AppendUint16(b, 2)
	b = binary.LittleEndian.AppendUint16(b, speech.WAVBits)
	b = append(b, []byte("data")...)
	b = binary.LittleEndian.AppendUint32(b, uint32(dataLen))
	b = append(b, make([]byte, dataLen)...)
	_, err := speech.ValidateWAV(b)
	return b, err
}
