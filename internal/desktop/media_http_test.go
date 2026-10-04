package desktop

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"path/filepath"
	"strings"
	"testing"

	"github.com/ProjectViVy/agent-diva/internal/speech"
)


const mediaHTTPWindowID = 7

// speechTestService wires a real speech service on temp dirs with a fake
// credential store and no provider traffic needed for route tests.
func speechTestService(t *testing.T, store *fakeSecretStore) *speech.Service {
	t.Helper()
	dir := t.TempDir()
	cfg, err := speech.OpenConfigStore(filepath.Join(dir, "speech.json"))
	if err != nil {
		t.Fatal(err)
	}
	assets, err := speech.OpenAssetStore(filepath.Join(dir, "assets"))
	if err != nil {
		t.Fatal(err)
	}
	return speech.NewService(cfg, speech.NewCredentialStore(store), assets, nil, true)
}

type fakeSecretStore struct{ m map[string]string }

func newFakeSecretStore() *fakeSecretStore { return &fakeSecretStore{m: map[string]string{}} }

func (s *fakeSecretStore) Set(service, user, secret string) error {
	s.m[service+"/"+user] = secret
	return nil
}
func (s *fakeSecretStore) Get(service, user string) (string, error) {
	return s.m[service+"/"+user], nil
}
func (s *fakeSecretStore) Delete(service, user string) error {
	delete(s.m, service+"/"+user)
	return nil
}

func grantReq(t *testing.T, method, url string, body []byte, cap *mediaCapability) *http.Request {
	t.Helper()
	req := httptest.NewRequest(method, url, bytes.NewReader(body))
	req.Header.Set("X-Diva-Media-Token", testToken)
	req.Header.Set(headerWindowID, fmt.Sprintf("%d", mediaHTTPWindowID))
	return req
}

const testToken = "tok-media-http"

func testCap() *mediaCapability { return newMediaCapability(mediaHTTPWindowID, testToken) }

func TestMediaVoiceAssetsRoute(t *testing.T) {
	svc := speechTestService(t, newFakeSecretStore())
	mux := NewMediaMux(nil, testCap(), svc)

	// Import via raw body + meta header.
	wav := testWAV(t)
	meta := `{"display_name":"clip","mime_type":"audio/wav"}`
	req := grantReq(t, "POST", "/media/voice-assets", wav, testCap())
	req.Header.Set("x-diva-asset-meta", meta)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != 200 {
		t.Fatalf("import: %d %s", rec.Code, rec.Body.String())
	}
	var desc struct {
		AssetID string `json:"asset_id"`
		Status  string `json:"status"`
	}
	if err := json.NewDecoder(rec.Body).Decode(&desc); err != nil || !strings.HasPrefix(desc.AssetID, "va-") {
		t.Fatalf("descriptor: %v %s", err, rec.Body.String())
	}

	// Read back raw bytes.
	req = grantReq(t, "GET", "/media/voice-assets/"+desc.AssetID, nil, testCap())
	rec = httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != 200 || rec.Header().Get("Content-Type") != "audio/wav" {
		t.Fatalf("read: %d %v", rec.Code, rec.Header())
	}
	got, _ := io.ReadAll(rec.Body)
	if !bytes.Equal(got, wav) {
		t.Fatal("read bytes differ")
	}

	// Foreign token denied before the body is read.
	req = httptest.NewRequest("GET", "/media/voice-assets/"+desc.AssetID, nil)
	req.Header.Set("X-Diva-Media-Token", "forged")
	req.Header.Set(headerWindowID, fmt.Sprintf("%d", mediaHTTPWindowID))
	rec = httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusForbidden {
		t.Fatalf("forged token: %d", rec.Code)
	}

	// Malformed ids (incl. traversal attempts) get a typed invalid_input
	// body — the path never reaches the store.
	for _, bad := range []string{"va-0", "va-../escape", "..%2F..%2Fetc"} {
		req = grantReq(t, "GET", "/media/voice-assets/"+bad, nil, testCap())
		rec = httptest.NewRecorder()
		mux.ServeHTTP(rec, req)
		if rec.Code == 200 {
			t.Fatalf("malformed id %q served", bad)
		}
	}
	req = grantReq(t, "GET", "/media/voice-assets/va-0", nil, testCap())
	rec = httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	var se speech.SpeechError
	_ = json.NewDecoder(rec.Body).Decode(&se)
	if se.Code != speech.CodeInvalidInput {
		t.Fatalf("typed error body: %s", rec.Body.String())
	}
}

func TestMediaSpeechMetaBounds(t *testing.T) {
	svc := speechTestService(t, newFakeSecretStore())
	mux := NewMediaMux(nil, testCap(), svc)

	// Missing meta → typed invalid_input.
	req := grantReq(t, "POST", "/media/speech/transcribe", testWAV(t), testCap())
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("no meta: %d", rec.Code)
	}

	// Over-2KiB meta → rejected.
	big := strings.Repeat("x", 3000)
	req = grantReq(t, "POST", "/media/speech/transcribe", testWAV(t), testCap())
	req.Header.Set("x-diva-speech-meta", big)
	rec = httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("big meta: %d", rec.Code)
	}
}

func TestMediaTranscribeNotConfigured(t *testing.T) {
	svc := speechTestService(t, newFakeSecretStore())
	mux := NewMediaMux(nil, testCap(), svc)
	_, _ = svc.SetContext("sess-1", 1)

	meta := `{"identity":{"request_id":"r-1","session_id":"sess-1","utterance_id":"u","generation":1},"mime_type":"audio/wav"}`
	req := grantReq(t, "POST", "/media/speech/transcribe", testWAV(t), testCap())
	req.Header.Set("x-diva-speech-meta", meta)
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	// No credential → typed not_configured (HTTP 404 body carries the code).
	if rec.Code == 200 {
		t.Fatalf("unconfigured transcribe succeeded: %s", rec.Body.String())
	}
	var se speech.SpeechError
	if err := json.NewDecoder(rec.Body).Decode(&se); err != nil || se.Code != speech.CodeNotConfigured {
		t.Fatalf("error body: %s", rec.Body.String())
	}
}

// testWAV is a minimal valid 16 kHz mono PCM16 WAV.
func testWAV(t *testing.T) []byte {
	t.Helper()
	w, err := speechValidateWAVFixture()
	if err != nil {
		t.Fatal(err)
	}
	return w
}
