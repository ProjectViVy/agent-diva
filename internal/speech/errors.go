// Package speech — DN-W4 Go port of the retired diva-speech crate:
// versioned preferences with revision CAS, OS-credential slots
// (presence-only readback, no plaintext fallback), bounded owned
// reference-asset storage with delete leases, closed provider adapters
// and one cancellable STT/TTS request lane.
package speech

// SpeechCode is the `diva.speech/v1` failure code set reachable from the
// native speech surface — serialized as snake_case strings.
type SpeechCode string

const (
	CodeNativeUnavailable      SpeechCode = "native_unavailable"
	CodeNotConfigured          SpeechCode = "not_configured"
	CodeDeviceUnavailable      SpeechCode = "device_unavailable"
	CodeCredentialUnavailable  SpeechCode = "credential_unavailable"
	CodeProviderError          SpeechCode = "provider_error"
	CodeInvalidAudio           SpeechCode = "invalid_audio"
	CodeUnsupportedReference   SpeechCode = "unsupported_reference"
	CodeAssetNotFound          SpeechCode = "asset_not_found"
	CodeStaleContext           SpeechCode = "stale_context"
	CodeInvalidInput           SpeechCode = "invalid_input"
	CodeRevisionConflict       SpeechCode = "revision_conflict"
	CodeBusy                   SpeechCode = "busy"
	CodeCancelled              SpeechCode = "cancelled"
	CodeTimeout                SpeechCode = "timeout"
)

// SpeechError is the failure body surfaced to the frontend. Secrets,
// audio bytes, raw text and provider URLs never appear in Message.
type SpeechError struct {
	Code       SpeechCode `json:"code"`
	Message    string     `json:"message"`
	Retryable  bool       `json:"retryable"`
	Provider   string     `json:"provider,omitempty"`
	HTTPStatus int        `json:"http_status,omitempty"`
}

func (e *SpeechError) Error() string { return string(e.Code) + ": " + e.Message }

func speechErr(code SpeechCode, msg string) *SpeechError {
	return &SpeechError{Code: code, Message: msg}
}

func (e *SpeechError) retryable(v bool) *SpeechError  { e.Retryable = v; return e }
func (e *SpeechError) provider(p Provider) *SpeechError { e.Provider = p.String(); return e }
func (e *SpeechError) httpStatus(s int) *SpeechError  { e.HTTPStatus = s; return e }

// CodeOf extracts the speech code for wire mapping; non-speech errors map
// to internal.
func CodeOf(err error) SpeechCode {
	if se, ok := err.(*SpeechError); ok {
		return se.Code
	}
	return "internal"
}
