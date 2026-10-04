package desktop

// W4 binary media adapter on the W0-proven internal Wails route. Token +
// window capability checked BEFORE expensive bodies are read; typed JSON
// error bodies on non-success statuses; no listener daemon, no raw
// filesystem path, no browser provider endpoint.

import (
	"bytes"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"

	"github.com/ProjectViVy/agent-diva/internal/speech"
)

// maxSpeechMetaBytes is the 2 KiB bound for the metadata headers
// (x-diva-speech-meta / x-diva-asset-meta).
const maxSpeechMetaBytes = 2 << 10

// speechErrorBody writes a typed diva.speech/v1 failure body. The frontend
// maps non-2xx to its own error shape; the body keeps the speech code.
func speechErrorBody(w http.ResponseWriter, status int, err error) {
	se, ok := err.(*speech.SpeechError)
	if !ok {
		se = &speech.SpeechError{Code: "internal", Message: fmt.Sprintf("%v", err)}
	}
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(se)
}

// statusForSpeech maps speech codes onto HTTP statuses.
func statusForSpeech(err error) int {
	se, ok := err.(*speech.SpeechError)
	if !ok {
		return http.StatusInternalServerError
	}
	switch se.Code {
	case speech.CodeInvalidAudio, speech.CodeInvalidInput, speech.CodeUnsupportedReference:
		return http.StatusBadRequest
	case speech.CodeAssetNotFound, speech.CodeNotConfigured:
		return http.StatusNotFound
	case speech.CodeStaleContext, speech.CodeRevisionConflict:
		return http.StatusConflict
	case speech.CodeBusy:
		return http.StatusConflict
	case speech.CodeCancelled:
		return 499
	case speech.CodeTimeout, speech.CodeProviderError, speech.CodeCredentialUnavailable:
		return http.StatusBadGateway
	case speech.CodeNativeUnavailable, speech.CodeDeviceUnavailable:
		return http.StatusServiceUnavailable
	default:
		return http.StatusInternalServerError
	}
}

type speechMeta struct {
	Identity speech.Identity `json:"identity"`
	MimeType string          `json:"mime_type,omitempty"`
}

func readSpeechMeta(r *http.Request) (*speechMeta, error) {
	raw := r.Header.Get("x-diva-speech-meta")
	if raw == "" || len(raw) > maxSpeechMetaBytes {
		return nil, &speech.SpeechError{Code: speech.CodeInvalidInput,
			Message: "x-diva-speech-meta missing or over 2 KiB"}
	}
	var meta speechMeta
	dec := json.NewDecoder(bytes.NewReader([]byte(raw)))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&meta); err != nil {
		return nil, &speech.SpeechError{Code: speech.CodeInvalidInput,
			Message: fmt.Sprintf("x-diva-speech-meta: %v", err)}
	}
	return &meta, nil
}

func readAssetMeta(r *http.Request) (*speech.AssetImportMeta, error) {
	raw := r.Header.Get("x-diva-asset-meta")
	if raw == "" || len(raw) > maxSpeechMetaBytes {
		return nil, &speech.SpeechError{Code: speech.CodeInvalidInput,
			Message: "x-diva-asset-meta missing or over 2 KiB"}
	}
	var meta speech.AssetImportMeta
	dec := json.NewDecoder(bytes.NewReader([]byte(raw)))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&meta); err != nil {
		return nil, &speech.SpeechError{Code: speech.CodeInvalidInput,
			Message: fmt.Sprintf("x-diva-asset-meta: %v", err)}
	}
	return &meta, nil
}

// registerSpeechMediaRoutes adds the W3-3 speech/voice-asset routes to mux.
// The capability grant is checked before any expensive body is read.
func registerSpeechMediaRoutes(mux *http.ServeMux, svc *speech.Service, cap *mediaCapability) {
	mux.HandleFunc("POST /media/speech/transcribe", func(w http.ResponseWriter, r *http.Request) {
		if !cap.grant(r) {
			speechErrorBody(w, http.StatusForbidden,
				&speech.SpeechError{Code: speech.CodeNativeUnavailable, Message: "media capability denied"})
			return
		}
		meta, err := readSpeechMeta(r)
		if err != nil {
			speechErrorBody(w, statusForSpeech(err), err)
			return
		}
		if meta.MimeType != "" && meta.MimeType != "audio/wav" {
			speechErrorBody(w, http.StatusBadRequest,
				&speech.SpeechError{Code: speech.CodeInvalidAudio, Message: "speech_transcribe accepts audio/wav"})
			return
		}
		r.Body = http.MaxBytesReader(w, r.Body, speech.WAVMaxBytes)
		body, err := io.ReadAll(r.Body)
		if err != nil {
			var maxErr *http.MaxBytesError
			if errors.As(err, &maxErr) {
				speechErrorBody(w, http.StatusRequestEntityTooLarge,
					&speech.SpeechError{Code: speech.CodeInvalidAudio, Message: "payload exceeds 8 MiB"})
				return
			}
			speechErrorBody(w, http.StatusBadRequest, err)
			return
		}
		reply, err := svc.Transcribe(&meta.Identity, body)
		if err != nil {
			speechErrorBody(w, statusForSpeech(err), err)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(reply)
	})

	mux.HandleFunc("POST /media/speech/synthesize", func(w http.ResponseWriter, r *http.Request) {
		if !cap.grant(r) {
			speechErrorBody(w, http.StatusForbidden,
				&speech.SpeechError{Code: speech.CodeNativeUnavailable, Message: "media capability denied"})
			return
		}
		r.Body = http.MaxBytesReader(w, r.Body, speech.TTSTextMaxBytes+4096)
		body, err := io.ReadAll(r.Body)
		if err != nil {
			speechErrorBody(w, http.StatusRequestEntityTooLarge,
				&speech.SpeechError{Code: speech.CodeInvalidInput, Message: "request body too large"})
			return
		}
		var req struct {
			Identity speech.Identity `json:"identity"`
			Text     string          `json:"text"`
		}
		dec := json.NewDecoder(bytes.NewReader(body))
		dec.DisallowUnknownFields()
		if err := dec.Decode(&req); err != nil {
			speechErrorBody(w, http.StatusBadRequest,
				&speech.SpeechError{Code: speech.CodeInvalidInput, Message: fmt.Sprintf("synthesize body: %v", err)})
			return
		}
		mp3, err := svc.Synthesize(&req.Identity, req.Text)
		if err != nil {
			speechErrorBody(w, statusForSpeech(err), err)
			return
		}
		w.Header().Set("Content-Type", "audio/mpeg")
		w.Header().Set("Content-Length", fmt.Sprintf("%d", len(mp3)))
		_, _ = w.Write(mp3)
	})

	mux.HandleFunc("POST /media/voice-assets", func(w http.ResponseWriter, r *http.Request) {
		if !cap.grant(r) {
			speechErrorBody(w, http.StatusForbidden,
				&speech.SpeechError{Code: speech.CodeNativeUnavailable, Message: "media capability denied"})
			return
		}
		meta, err := readAssetMeta(r)
		if err != nil {
			speechErrorBody(w, statusForSpeech(err), err)
			return
		}
		r.Body = http.MaxBytesReader(w, r.Body, speech.AssetMaxFileBytes)
		body, err := io.ReadAll(r.Body)
		if err != nil {
			var maxErr *http.MaxBytesError
			if errors.As(err, &maxErr) {
				speechErrorBody(w, http.StatusRequestEntityTooLarge,
					&speech.SpeechError{Code: speech.CodeInvalidAudio, Message: "asset exceeds 10 MiB"})
				return
			}
			speechErrorBody(w, http.StatusBadRequest, err)
			return
		}
		desc, err := svc.Assets().Import(body, *meta)
		if err != nil {
			speechErrorBody(w, statusForSpeech(err), err)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(desc)
	})

	mux.HandleFunc("GET /media/voice-assets/{id}", func(w http.ResponseWriter, r *http.Request) {
		if !cap.grant(r) {
			speechErrorBody(w, http.StatusForbidden,
				&speech.SpeechError{Code: speech.CodeNativeUnavailable, Message: "media capability denied"})
			return
		}
		lease, err := svc.Assets().Read(r.PathValue("id"))
		if err != nil {
			speechErrorBody(w, statusForSpeech(err), err)
			return
		}
		defer lease.Release()
		w.Header().Set("Content-Type", lease.MimeType)
		w.Header().Set("Content-Length", fmt.Sprintf("%d", len(lease.Bytes)))
		_, _ = w.Write(lease.Bytes)
	})
}
