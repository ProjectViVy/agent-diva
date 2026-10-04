package desktop

// W4 dispatch table: the JSON (non-binary) retained native commands,
// routed to the Go speech service. Binary payloads stay on the media
// routes (media_http.go). Unknown commands still answer not_ready.

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"

	"github.com/ProjectViVy/agent-diva/internal/speech"
)

// speechConfigReadback is the speech_config_get/update shape — presence
// only, never key material.
type speechConfigReadback struct {
	Schema          string                    `json:"schema"`
	Revision        uint64                    `json:"revision"`
	Preferences     speech.Preferences        `json:"preferences"`
	CredentialState map[string]string         `json:"credential_state"`
	WindowContext   *speech.Context           `json:"window_context"`
}

func speechDispatch(svc *speech.Service) map[string]func(context.Context, json.RawMessage) (json.RawMessage, error) {
	readback := func() (json.RawMessage, error) {
		var rb speechConfigReadback
		svc.Config(func(cs *speech.ConfigStore) {
			rb.Schema = speech.SpeechSchema
			rb.Revision = cs.Revision()
			rb.Preferences = cs.Preferences()
		})
		rb.CredentialState = map[string]string{
			"siliconflow": string(svc.Credentials().Presence(speech.ProviderSiliconFlow)),
			"minimax":     string(svc.Credentials().Presence(speech.ProviderMiniMax)),
		}
		rb.WindowContext = svc.CurrentContext()
		return json.Marshal(rb)
	}

	return map[string]func(context.Context, json.RawMessage) (json.RawMessage, error){
		"speech_config_get": func(_ context.Context, _ json.RawMessage) (json.RawMessage, error) {
			return readback()
		},

		"speech_config_update": func(_ context.Context, payload json.RawMessage) (json.RawMessage, error) {
			var req struct {
				BaseRevision uint64             `json:"base_revision"`
				Preferences  speech.Preferences `json:"preferences"`
			}
			if err := strictJSON(payload, &req); err != nil {
				return nil, speechErrOf(speech.CodeInvalidInput, fmt.Sprintf("payload: %v", err))
			}
			var updErr error
			svc.Config(func(cs *speech.ConfigStore) {
				_, updErr = cs.Update(req.BaseRevision, req.Preferences, svc.Assets().Exists)
			})
			if updErr != nil {
				return nil, updErr
			}
			return readback()
		},

		"speech_credential_set": func(_ context.Context, payload json.RawMessage) (json.RawMessage, error) {
			var req struct {
				Provider string `json:"provider"`
				Key      string `json:"key"`
			}
			if err := strictJSON(payload, &req); err != nil {
				return nil, speechErrOf(speech.CodeInvalidInput, fmt.Sprintf("payload: %v", err))
			}
			p, err := speech.ParseProvider(req.Provider)
			if err != nil {
				return nil, err
			}
			if err := svc.Credentials().Set(p, req.Key); err != nil {
				return nil, err
			}
			return json.Marshal(map[string]any{
				"provider": p.String(),
				"present":  svc.Credentials().Presence(p) == speech.PresencePresent,
			})
		},

		"speech_credential_delete": func(_ context.Context, payload json.RawMessage) (json.RawMessage, error) {
			var req struct {
				Provider string `json:"provider"`
			}
			if err := strictJSON(payload, &req); err != nil {
				return nil, speechErrOf(speech.CodeInvalidInput, fmt.Sprintf("payload: %v", err))
			}
			p, err := speech.ParseProvider(req.Provider)
			if err != nil {
				return nil, err
			}
			if err := svc.Credentials().Delete(p); err != nil {
				return nil, err
			}
			return json.Marshal(map[string]any{
				"provider": p.String(),
				"present":  svc.Credentials().Presence(p) == speech.PresencePresent,
			})
		},

		"speech_context_set": func(_ context.Context, payload json.RawMessage) (json.RawMessage, error) {
			var req struct {
				SessionID  string `json:"session_id"`
				Generation uint64 `json:"generation"`
			}
			if err := strictJSON(payload, &req); err != nil {
				return nil, speechErrOf(speech.CodeInvalidInput, fmt.Sprintf("payload: %v", err))
			}
			ctx, err := svc.SetContext(req.SessionID, req.Generation)
			if err != nil {
				return nil, err
			}
			return json.Marshal(ctx)
		},

		"speech_cancel": func(_ context.Context, payload json.RawMessage) (json.RawMessage, error) {
			var req struct {
				RequestID string `json:"request_id"`
			}
			if err := strictJSON(payload, &req); err != nil {
				return nil, speechErrOf(speech.CodeInvalidInput, fmt.Sprintf("payload: %v", err))
			}
			return json.Marshal(map[string]string{
				"request_id": req.RequestID,
				"status":     svc.Cancel(req.RequestID),
			})
		},

		"voice_asset_list": func(_ context.Context, _ json.RawMessage) (json.RawMessage, error) {
			assets, err := svc.Assets().List()
			if err != nil {
				return nil, err
			}
			return json.Marshal(assets)
		},

		"voice_asset_delete": func(_ context.Context, payload json.RawMessage) (json.RawMessage, error) {
			var req struct {
				AssetID string `json:"asset_id"`
			}
			if err := strictJSON(payload, &req); err != nil {
				return nil, speechErrOf(speech.CodeInvalidInput, fmt.Sprintf("payload: %v", err))
			}
			status, err := svc.Assets().Delete(req.AssetID)
			if err != nil {
				return nil, err
			}
			return json.Marshal(status)
		},
	}
}

func strictJSON(data []byte, v any) error {
	dec := json.NewDecoder(bytes.NewReader(data))
	dec.DisallowUnknownFields()
	return dec.Decode(v)
}

func speechErrOf(code speech.SpeechCode, msg string) *speech.SpeechError {
	return &speech.SpeechError{Code: code, Message: msg}
}
