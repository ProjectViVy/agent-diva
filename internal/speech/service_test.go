package speech

import (
	"context"
	"encoding/hex"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// fakeStore is an in-memory SecretStore; fail=true models a locked
// platform store (credential_unavailable, retryable).
type fakeStore struct {
	m    map[string]string
	fail bool
}

func newFakeStore() *fakeStore { return &fakeStore{m: map[string]string{}} }

func (s *fakeStore) key(service, user string) string { return service + "/" + user }

func (s *fakeStore) Set(service, user, secret string) error {
	if s.fail {
		return mapStoreError(errStoreDown)
	}
	s.m[s.key(service, user)] = secret
	return nil
}

func (s *fakeStore) Get(service, user string) (string, error) {
	if s.fail {
		return "", mapStoreError(errStoreDown)
	}
	return s.m[s.key(service, user)], nil
}

func (s *fakeStore) Delete(service, user string) error {
	if s.fail {
		return mapStoreError(errStoreDown)
	}
	delete(s.m, s.key(service, user))
	return nil
}

var errStoreDown = fmt.Errorf("store down")

func newTestService(t *testing.T, store *fakeStore, handler http.Handler) (*Service, *httptest.Server) {
	t.Helper()
	dir := t.TempDir()
	cfg, err := OpenConfigStore(filepath.Join(dir, "speech.json"))
	if err != nil {
		t.Fatal(err)
	}
	assets, err := OpenAssetStore(filepath.Join(dir, "assets"))
	if err != nil {
		t.Fatal(err)
	}
	srv := httptest.NewServer(handler)
	svc := NewService(cfg, NewCredentialStore(store), assets, nil, true)
	// Point every provider block at the fixture server.
	svc.Config(func(cs *ConfigStore) {
		p := cs.Preferences()
		p.STT.BaseURL = srv.URL
		p.TTS.SiliconFlow.BaseURL = srv.URL
		if _, err := cs.Update(cs.Revision(), p, assets.Exists); err != nil {
			t.Fatal(err)
		}
	})
	return svc, srv
}

func testIdentity() *Identity {
	return &Identity{RequestID: "r-1", SessionID: "sess-1", UtteranceID: "u-1", Generation: 1}
}

func TestConfigStoreCAS(t *testing.T) {
	dir := t.TempDir()
	cs, err := OpenConfigStore(filepath.Join(dir, "speech.json"))
	if err != nil {
		t.Fatal(err)
	}
	if cs.Revision() != 0 {
		t.Fatalf("seed revision %d", cs.Revision())
	}
	p := cs.Preferences()
	rev, err := cs.Update(0, p, func(string) bool { return true })
	if err != nil || rev != 1 {
		t.Fatalf("update: %v rev=%d", err, rev)
	}
	if _, err := cs.Update(0, p, func(string) bool { return true }); err == nil ||
		codeOf(t, err) != CodeRevisionConflict {
		t.Fatalf("stale base accepted: %v", err)
	}
	// Corrupt file is an explicit error, never silently re-seeded.
	bad := filepath.Join(dir, "bad.json")
	if err := os.WriteFile(bad, []byte(`{corrupt`), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, err := OpenConfigStore(bad); err == nil {
		t.Fatal("corrupt file opened")
	}
}

func TestConfigValidation(t *testing.T) {
	cs, err := OpenConfigStore(filepath.Join(t.TempDir(), "speech.json"))
	if err != nil {
		t.Fatal(err)
	}
	p := cs.Preferences()
	p.STT.BaseURL = "ftp://example.com"
	if _, err := cs.Update(0, p, func(string) bool { return true }); codeOf(t, err) != CodeInvalidInput {
		t.Fatalf("scheme: %v", err)
	}
	p = cs.Preferences()
	p.TTS.Provider = ProviderMiniMax
	p.TTS.MiniMax = nil
	if _, err := cs.Update(0, p, func(string) bool { return true }); codeOf(t, err) != CodeInvalidInput {
		t.Fatalf("missing minimax block: %v", err)
	}
	p = cs.Preferences()
	p.TTS.SiliconFlow.Speed = 9
	if _, err := cs.Update(0, p, func(string) bool { return true }); codeOf(t, err) != CodeInvalidInput {
		t.Fatalf("speed bound: %v", err)
	}
	// Inline reference to a missing asset.
	p = cs.Preferences()
	p.TTS.SiliconFlow.Reference = &Reference{AssetID: "va-0123456789abcdef", Transcript: "hi"}
	if _, err := cs.Update(0, p, func(string) bool { return false }); codeOf(t, err) != CodeAssetNotFound {
		t.Fatalf("missing asset ref: %v", err)
	}
}

func TestCredentials(t *testing.T) {
	store := newFakeStore()
	creds := NewCredentialStore(store)
	if creds.Presence(ProviderSiliconFlow) != PresenceAbsent {
		t.Fatal("fresh store not absent")
	}
	if err := creds.Set(ProviderSiliconFlow, "  "); codeOf(t, err) != CodeInvalidInput {
		t.Fatalf("empty key: %v", err)
	}
	if err := creds.Set(ProviderSiliconFlow, strings.Repeat("k", 513)); codeOf(t, err) != CodeInvalidInput {
		t.Fatalf("overlong key: %v", err)
	}
	if err := creds.Set(ProviderSiliconFlow, "sk-test"); err != nil {
		t.Fatal(err)
	}
	if creds.Presence(ProviderSiliconFlow) != PresencePresent {
		t.Fatal("not present after set")
	}
	if err := creds.Delete(ProviderSiliconFlow); err != nil {
		t.Fatal(err)
	}
	if creds.Presence(ProviderSiliconFlow) != PresenceAbsent {
		t.Fatal("not absent after delete")
	}
	// Unavailable store → credential_unavailable, never plaintext fallback.
	store.fail = true
	if creds.Presence(ProviderSiliconFlow) != PresenceUnavailable {
		t.Fatal("down store not unavailable")
	}
	if err := creds.Set(ProviderSiliconFlow, "x"); codeOf(t, err) != CodeCredentialUnavailable {
		t.Fatalf("set on down store: %v", err)
	}
}

func TestAssetStore(t *testing.T) {
	dir := t.TempDir()
	assets, err := OpenAssetStore(filepath.Join(dir, "assets"))
	if err != nil {
		t.Fatal(err)
	}
	wav := makeWAV(0.01)
	meta := AssetImportMeta{DisplayName: "sample", MimeType: "audio/wav"}
	desc, err := assets.Import(wav, meta)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.HasPrefix(desc.AssetID, "va-") || desc.Status != "active" {
		t.Fatalf("bad descriptor: %+v", desc)
	}
	// MIME/bytes mismatch rejected.
	if _, err := assets.Import(wav, AssetImportMeta{DisplayName: "x", MimeType: "audio/mpeg"}); codeOf(t, err) != CodeInvalidAudio {
		t.Fatalf("mime mismatch: %v", err)
	}
	// Re-import same bytes dedups and renames.
	desc2, err := assets.Import(wav, AssetImportMeta{DisplayName: "renamed", MimeType: "audio/wav"})
	if err != nil || desc2.AssetID != desc.AssetID || desc2.DisplayName != "renamed" {
		t.Fatalf("reimport: %v %+v", err, desc2)
	}
	// Lease then delete → pending; release completes removal.
	lease, err := assets.Read(desc.AssetID)
	if err != nil {
		t.Fatal(err)
	}
	status, err := assets.Delete(desc.AssetID)
	if err != nil || status != "pending" {
		t.Fatalf("delete under lease: %v %q", err, status)
	}
	if assets.Exists(desc.AssetID) {
		t.Fatal("pending delete still counts for config")
	}
	lease.Release()
	if _, err := assets.Read(desc.AssetID); codeOf(t, err) != CodeAssetNotFound {
		t.Fatalf("read after pending-delete release: %v", err)
	}
	// Caps: file size.
	if _, err := assets.Import(make([]byte, AssetMaxFileBytes+1), AssetImportMeta{DisplayName: "big", MimeType: "audio/wav"}); codeOf(t, err) != CodeInvalidAudio {
		t.Fatalf("oversize import: %v", err)
	}
	// File count cap.
	for i := 0; i < AssetMaxFiles; i++ {
		b := append([]byte("ID3"), make([]byte, 64+i)...)
		if _, err := assets.Import(b, AssetImportMeta{DisplayName: fmt.Sprintf("a%d", i), MimeType: "audio/mpeg"}); err != nil {
			t.Fatalf("import %d: %v", i, err)
		}
	}
	b := append([]byte("ID3"), make([]byte, 4096)...)
	if _, err := assets.Import(b, AssetImportMeta{DisplayName: "over", MimeType: "audio/mpeg"}); codeOf(t, err) != CodeInvalidInput {
		t.Fatalf("count cap: %v", err)
	}
	// Malformed ids never reach the filesystem.
	for _, bad := range []string{"va-../etc", "va-0123456789ABCDEF", "va-0", "../etc/passwd", "va-0123456789abcdef/extra"} {
		if _, err := assets.Read(bad); err == nil {
			t.Fatalf("malformed id %q read", bad)
		}
	}
	// Reopen reconciles: manifest rows without files are dropped.
	dir2 := t.TempDir()
	assets2, err := OpenAssetStore(filepath.Join(dir2, "a"))
	if err != nil {
		t.Fatal(err)
	}
	d, err := assets2.Import(wav, meta)
	if err != nil {
		t.Fatal(err)
	}
	// Delete the bytes out from under the manifest.
	entries, _ := filepath.Glob(filepath.Join(dir2, "a", "va-*"))
	for _, e := range entries {
		if err := os.Remove(e); err != nil {
			t.Fatal(err)
		}
	}
	assets3, err := OpenAssetStore(filepath.Join(dir2, "a"))
	if err != nil {
		t.Fatal(err)
	}
	if assets3.Exists(d.AssetID) {
		t.Fatal("orphan manifest row kept")
	}
}

// --- provider fixtures -----------------------------------------------------

func sttFixture(t *testing.T, reply string, status int) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/v1/audio/transcriptions":
			if r.Header.Get("Authorization") == "" {
				t.Error("missing bearer")
			}
			w.WriteHeader(status)
			fmt.Fprintf(w, `{"text":%q}`, reply)
		case "/v1/audio/speech":
			w.Write(mp3Fixture())
		case "/v1/t2a_v2":
			hexAudio := hex.EncodeToString(mp3Fixture())
			fmt.Fprintf(w, `{"base_resp":{"status_code":0},"data":{"status":2,"audio":%q}}`, hexAudio)
		default:
			w.WriteHeader(404)
		}
	})
}

func mp3Fixture() []byte {
	return append([]byte("ID3"), make([]byte, 512)...)
}

func TestTranscribeSettle(t *testing.T) {
	var diags []Diagnostic
	store := newFakeStore()
	_ = store.Set(credentialService, credentialSlot(ProviderSiliconFlow), "sk-x")
	svc, srv := newTestService(t, store, sttFixture(t, "hello world", 200))
	defer srv.Close()
	svc.diagnostic = func(d Diagnostic) { diags = append(diags, d) }
	if _, err := svc.SetContext("sess-1", 1); err != nil {
		t.Fatal(err)
	}
	reply, err := svc.Transcribe(testIdentity(), makeWAV(1))
	if err != nil {
		t.Fatal(err)
	}
	if reply.Status != "transcribed" || reply.Text != "hello world" {
		t.Fatalf("reply: %+v", reply)
	}
	if svc.ActiveRequests() != 0 {
		t.Fatal("slot leaked")
	}
	var admitted, settled int
	for _, d := range diags {
		if d.Phase == "admitted" {
			admitted++
		}
		if d.Phase == "settled" {
			settled++
			if d.Bytes == 0 {
				t.Fatal("settled diagnostic lost byte count")
			}
		}
	}
	if admitted != 1 || settled != 1 {
		t.Fatalf("diagnostics: admitted=%d settled=%d", admitted, settled)
	}
}

func TestTranscribeNoSpeech(t *testing.T) {
	store := newFakeStore()
	_ = store.Set(credentialService, credentialSlot(ProviderSiliconFlow), "sk-x")
	svc, srv := newTestService(t, store, sttFixture(t, "  ", 200))
	defer srv.Close()
	_, _ = svc.SetContext("sess-1", 1)
	reply, err := svc.Transcribe(testIdentity(), makeWAV(1))
	if err != nil || reply.Status != "no_speech" || reply.Text != "" {
		t.Fatalf("no_speech: %v %+v", err, reply)
	}
}

func TestTranscribeNoCredential(t *testing.T) {
	svc, srv := newTestService(t, newFakeStore(), sttFixture(t, "x", 200))
	defer srv.Close()
	_, _ = svc.SetContext("sess-1", 1)
	if _, err := svc.Transcribe(testIdentity(), makeWAV(1)); codeOf(t, err) != CodeNotConfigured {
		t.Fatalf("no credential: %v", err)
	}
	if svc.ActiveRequests() != 0 {
		t.Fatal("slot leaked on prepare failure")
	}
}

func TestAdmissionGuards(t *testing.T) {
	store := newFakeStore()
	_ = store.Set(credentialService, credentialSlot(ProviderSiliconFlow), "sk-x")
	svc, srv := newTestService(t, store, sttFixture(t, "x", 200))
	defer srv.Close()
	// No context → not_configured.
	if _, err := svc.Transcribe(testIdentity(), makeWAV(1)); codeOf(t, err) != CodeNotConfigured {
		t.Fatalf("no context: %v", err)
	}
	_, _ = svc.SetContext("sess-1", 1)
	// Stale identity → stale_context.
	stale := &Identity{RequestID: "r-2", SessionID: "other", UtteranceID: "u", Generation: 1}
	if _, err := svc.Transcribe(stale, makeWAV(1)); codeOf(t, err) != CodeStaleContext {
		t.Fatalf("stale identity: %v", err)
	}
	// Older generation rejected; newer advances and aborts the old.
	if _, err := svc.SetContext("sess-1", 1); err != nil {
		t.Fatal("idempotent set_context")
	}
	if _, err := svc.SetContext("sess-1", 0); codeOf(t, err) != CodeStaleContext {
		t.Fatalf("older generation: %v", err)
	}
	if _, err := svc.SetContext("sess-2", 2); err != nil {
		t.Fatal(err)
	}
	if _, err := svc.Transcribe(testIdentity(), makeWAV(1)); codeOf(t, err) != CodeStaleContext {
		t.Fatalf("old-context request: %v", err)
	}
}

func TestCancelDuringProviderCall(t *testing.T) {
	release := make(chan struct{})
	slow := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		select {
		case <-release:
			w.Write(mp3Fixture())
		case <-r.Context().Done():
			return
		}
	})
	store := newFakeStore()
	_ = store.Set(credentialService, credentialSlot(ProviderSiliconFlow), "sk-x")
	svc, srv := newTestService(t, store, slow)
	defer srv.Close()
	defer close(release)
	_, _ = svc.SetContext("sess-1", 1)
	done := make(chan error, 1)
	go func() {
		id := testIdentity()
		_, err := svc.Synthesize(id, "hello")
		done <- err
	}()
	// Wait for admission then cancel.
	deadline := time.Now().Add(2 * time.Second)
	for svc.ActiveRequests() == 0 && time.Now().Before(deadline) {
		time.Sleep(5 * time.Millisecond)
	}
	if svc.Cancel("r-1") != "cancelled" {
		t.Fatal("cancel did not hit in-flight request")
	}
	select {
	case err := <-done:
		if codeOf(t, err) != CodeCancelled {
			t.Fatalf("cancelled request returned: %v", err)
		}
	case <-time.After(5 * time.Second):
		t.Fatal("cancelled request hung")
	}
	if svc.Cancel("r-1") != "settled" {
		t.Fatal("idempotent cancel")
	}
}

func TestLateSuccessDiscarded(t *testing.T) {
	var diags []Diagnostic
	gate := make(chan struct{})
	hooked := make(chan struct{}, 1)
	store := newFakeStore()
	_ = store.Set(credentialService, credentialSlot(ProviderSiliconFlow), "sk-x")
	svc, srv := newTestService(t, store, sttFixture(t, "late text", 200))
	defer srv.Close()
	svc.diagnostic = func(d Diagnostic) { diags = append(diags, d) }
	// Pre-settle hook blocks until the context has advanced — the provider
	// success is provably late.
	svc.SetPreSettleHook(func() {
		close(hooked)
		<-gate
	})
	_, _ = svc.SetContext("sess-1", 1)
	done := make(chan error, 1)
	go func() {
		_, err := svc.Transcribe(testIdentity(), makeWAV(1))
		done <- err
	}()
	<-hooked
	// Context advances while the provider result is already in hand.
	if _, err := svc.SetContext("sess-1", 2); err != nil {
		t.Fatal(err)
	}
	close(gate)
	if err := <-done; codeOf(t, err) != CodeStaleContext {
		t.Fatalf("late success not discarded: %v", err)
	}
	var discarded bool
	for _, d := range diags {
		if d.Phase == "discarded" {
			discarded = true
		}
	}
	if !discarded {
		t.Fatal("no discarded diagnostic")
	}
}

func TestSynthesizeSiliconFlow(t *testing.T) {
	store := newFakeStore()
	_ = store.Set(credentialService, credentialSlot(ProviderSiliconFlow), "sk-x")
	svc, srv := newTestService(t, store, sttFixture(t, "", 200))
	defer srv.Close()
	_, _ = svc.SetContext("sess-1", 1)
	mp3, err := svc.Synthesize(testIdentity(), "hello")
	if err != nil {
		t.Fatal(err)
	}
	if !mp3Magic(mp3) {
		t.Fatal("non-mp3 accepted")
	}
}

func TestSynthesizeMiniMax(t *testing.T) {
	store := newFakeStore()
	_ = store.Set(credentialService, credentialSlot(ProviderMiniMax), "sk-x")
	svc, srv := newTestService(t, store, sttFixture(t, "", 200))
	defer srv.Close()
	svc.Config(func(cs *ConfigStore) {
		p := cs.Preferences()
		p.TTS.Provider = ProviderMiniMax
		p.TTS.MiniMax = &MiniMaxTts{BaseURL: srv.URL, Model: "speech-2.8-hd", VoiceID: "v1", Speed: 1, Volume: 1}
		if _, err := cs.Update(cs.Revision(), p, svc.Assets().Exists); err != nil {
			t.Fatal(err)
		}
	})
	_, _ = svc.SetContext("sess-1", 1)
	mp3, err := svc.Synthesize(testIdentity(), "hello")
	if err != nil || !mp3Magic(mp3) {
		t.Fatalf("minimax: %v", err)
	}
}

func TestProviderErrors(t *testing.T) {
	store := newFakeStore()
	_ = store.Set(credentialService, credentialSlot(ProviderSiliconFlow), "sk-x")
	svc, srv := newTestService(t, store, sttFixture(t, "", 500))
	defer srv.Close()
	_, _ = svc.SetContext("sess-1", 1)
	err := error(nil)
	_, err = svc.Transcribe(testIdentity(), makeWAV(1))
	se, ok := err.(*SpeechError)
	if !ok || se.Code != CodeProviderError || se.HTTPStatus != 500 || !se.Retryable {
		t.Fatalf("500: %+v", err)
	}
}

func TestShutdown(t *testing.T) {
	release := make(chan struct{})
	slow := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		select {
		case <-release:
		case <-r.Context().Done():
		}
		w.Write(mp3Fixture())
	})
	store := newFakeStore()
	_ = store.Set(credentialService, credentialSlot(ProviderSiliconFlow), "sk-x")
	svc, srv := newTestService(t, store, slow)
	defer srv.Close()
	defer close(release)
	_, _ = svc.SetContext("sess-1", 1)
	done := make(chan error, 1)
	go func() {
		_, err := svc.Synthesize(testIdentity(), "hello")
		done <- err
	}()
	deadline := time.Now().Add(2 * time.Second)
	for svc.ActiveRequests() == 0 && time.Now().Before(deadline) {
		time.Sleep(5 * time.Millisecond)
	}
	rep := svc.Shutdown(3 * time.Second)
	if rep.InflightAtStart != 1 {
		t.Fatalf("inflight: %+v", rep)
	}
	// The cancelled HTTP call must tear down: joined honestly.
	if rep.Remaining != 0 {
		t.Fatalf("unjoined task: %+v", rep)
	}
	if err := <-done; codeOf(t, err) != CodeCancelled {
		t.Fatalf("shutdown abort: %v", err)
	}
	// Post-shutdown admission rejected.
	if _, err := svc.Synthesize(testIdentity(), "again"); codeOf(t, err) != CodeCancelled {
		t.Fatalf("post-shutdown admit: %v", err)
	}
}

func TestShutdownContextHonorsTheCallerDeadline(t *testing.T) {
	svc := &Service{registry: NewRegistry()}
	blocked := make(chan struct{})
	svc.registry.Track("blocked-cleanup", blocked)
	ctx, cancel := context.WithTimeout(context.Background(), 50*time.Millisecond)
	defer cancel()
	started := time.Now()
	report := svc.ShutdownContext(ctx)
	if elapsed := time.Since(started); elapsed > 500*time.Millisecond {
		t.Fatalf("ShutdownContext took %s past its caller deadline", elapsed)
	}
	if report.InflightAtStart != 0 || report.Joined != 0 || report.Remaining != 1 {
		t.Fatalf("shutdown report = %+v, want one unjoined task", report)
	}
	close(blocked)
}

func TestSingleFlightSlot(t *testing.T) {
	release := make(chan struct{})
	slow := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		select {
		case <-release:
		case <-r.Context().Done():
		}
		w.Write(mp3Fixture())
	})
	store := newFakeStore()
	_ = store.Set(credentialService, credentialSlot(ProviderSiliconFlow), "sk-x")
	svc, srv := newTestService(t, store, slow)
	defer srv.Close()
	defer close(release)
	_, _ = svc.SetContext("sess-1", 1)
	done := make(chan error, 1)
	go func() {
		_, err := svc.Synthesize(testIdentity(), "one")
		done <- err
	}()
	deadline := time.Now().Add(2 * time.Second)
	for svc.ActiveRequests() == 0 && time.Now().Before(deadline) {
		time.Sleep(5 * time.Millisecond)
	}
	second := &Identity{RequestID: "r-2", SessionID: "sess-1", UtteranceID: "u", Generation: 1}
	if _, err := svc.Synthesize(second, "two"); codeOf(t, err) != CodeBusy {
		t.Fatalf("second tts admitted: %v", err)
	}
	svc.Cancel("r-1")
	<-done
}
