package speech

// DN-6B request lane: admission (trusted window + session + monotonic
// generation, one STT and one TTS slot), frozen config/credential/
// reference snapshots taken at admission, bounded provider HTTP, and
// abort/join cleanup on cancel, context advance, hide and quit. The
// registry mutex never spans keyring or HTTP work.

import (
	"context"
	"fmt"
	"net/http"
	"path/filepath"
	"sync"
	"time"
	"unicode/utf8"
)

const (
	metaIDMax      = 64
	metaSessionMax = 128
	metaFieldMax   = 128
)

// Identity is the C2-4 SpeechIdentity: correlation and fencing fields —
// never a VIVY authority grant.
type Identity struct {
	RequestID   string `json:"request_id"`
	SessionID   string `json:"session_id"`
	RunID       string `json:"run_id,omitempty"`
	UtteranceID string `json:"utterance_id"`
	Generation  uint64 `json:"generation"`
}

func validateID(field, value string, max int) error {
	if value == "" || len(value) > max {
		return invalidInput("%s empty or overlong", field)
	}
	for _, c := range value {
		if !(c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' || c >= '0' && c <= '9' ||
			c == '.' || c == '_' || c == '-' || c == ':') {
			return invalidInput("%s contains unsafe characters", field)
		}
	}
	return nil
}

// Validate checks identity bounds and character safety.
func (i *Identity) Validate() error {
	if err := validateID("request_id", i.RequestID, metaIDMax); err != nil {
		return err
	}
	if err := validateID("session_id", i.SessionID, metaSessionMax); err != nil {
		return err
	}
	if err := validateID("utterance_id", i.UtteranceID, metaFieldMax); err != nil {
		return err
	}
	if i.RunID != "" {
		if err := validateID("run_id", i.RunID, metaFieldMax); err != nil {
			return err
		}
	}
	return nil
}

// Diagnostic is the safe `speech:diagnostic` payload: phase/provider/
// safe code/HTTP status/elapsed/byte counts/identity. No secrets, audio,
// or text.
type Diagnostic struct {
	Phase          string     `json:"phase"`
	Kind           string     `json:"kind"`
	Provider       string     `json:"provider,omitempty"`
	Code           SpeechCode `json:"code,omitempty"`
	HTTPStatus     int        `json:"http_status,omitempty"`
	ElapsedMs      uint64     `json:"elapsed_ms"`
	Bytes          uint64     `json:"bytes"`
	RequestID      string     `json:"request_id"`
	UtteranceID    string     `json:"utterance_id"`
	Generation     uint64     `json:"generation"`
	ConfigRevision uint64     `json:"config_revision"`
}

// TranscribeReply is the `speech_transcribe` success shape — identity
// echoed back.
type TranscribeReply struct {
	Identity Identity `json:"identity"`
	Status   string   `json:"status"`
	Text     string   `json:"text"`
}

// ShutdownReport is honest teardown accounting for quit_with_inflight.
type ShutdownReport struct {
	InflightAtStart int `json:"inflight_at_start"`
	Joined          int `json:"joined"`
	Remaining       int `json:"remaining"`
}

// prepared is the frozen request snapshot taken at admission: nothing in
// it can be altered by a later config/credential/reference change.
type prepared struct {
	kind     RequestKind
	provider Provider
	sttCfg   *SttPreferences
	sfCfg    *SiliconFlowTts
	mmCfg    *MiniMaxTts
	key      string
	wav      []byte
	text     string
	inline   *AssetLease
}

type preparedOutcome struct {
	isText   bool
	text     string
	mp3      []byte
	bytes    uint64
	provider Provider
	hasProv  bool
}

// Service is the one cancellable speech service.
type Service struct {
	config                *serviceMutex[ConfigStore]
	credentials           *CredentialStore
	assets                *AssetStore
	registry              *Registry
	client                *http.Client
	allowInsecureLoopback bool
	diagnostic            func(Diagnostic)

	preSettleHookMu sync.Mutex
	preSettleHook   func()
}

// serviceMutex serializes a ConfigStore — Go has no Mutex<T>.
type serviceMutex[T any] struct {
	mu sync.Mutex
	v  *T
}

func (m *serviceMutex[T]) lock() *T { m.mu.Lock(); return m.v }
func (m *serviceMutex[T]) unlock()  { m.mu.Unlock() }

// OpenService is the production open: OS keyring, on-disk config/assets
// under dir. `diagnostic` receives every emitted speech:diagnostic
// payload.
func OpenService(dir string, diagnostic func(Diagnostic)) (*Service, error) {
	cfg, err := OpenConfigStore(filepath.Join(dir, "speech.json"))
	if err != nil {
		return nil, err
	}
	assets, err := OpenAssetStore(filepath.Join(dir, "assets"))
	if err != nil {
		return nil, err
	}
	return NewService(cfg, NewCredentialStore(OsKeyring{}), assets, diagnostic, false), nil
}

// NewService wires explicit parts — tests inject a fake SecretStore and
// allow insecure loopback for fixture servers.
func NewService(config *ConfigStore, credentials *CredentialStore, assets *AssetStore,
	diagnostic func(Diagnostic), allowInsecureLoopback bool) *Service {
	if diagnostic == nil {
		diagnostic = func(Diagnostic) {}
	}
	return &Service{
		config:                &serviceMutex[ConfigStore]{v: config},
		credentials:           credentials,
		assets:                assets,
		registry:              NewRegistry(),
		client:                newHTTPClient(),
		allowInsecureLoopback: allowInsecureLoopback,
		diagnostic:            diagnostic,
	}
}

// Config returns the serialized config store handle.
func (s *Service) Config(fn func(*ConfigStore)) {
	v := s.config.lock()
	defer s.config.unlock()
	fn(v)
}

func (s *Service) Credentials() *CredentialStore { return s.credentials }
func (s *Service) Assets() *AssetStore           { return s.assets }
func (s *Service) CurrentContext() *Context      { return s.registry.CurrentContext() }
func (s *Service) ActiveRequests() int           { return s.registry.ActiveCount() }

// SetPreSettleHook is the test seam: runs between the provider result
// and registry settle, letting a fixture make a completion provably late.
func (s *Service) SetPreSettleHook(hook func()) {
	s.preSettleHookMu.Lock()
	defer s.preSettleHookMu.Unlock()
	s.preSettleHook = hook
}

// SetContext is `speech_context_set`.
func (s *Service) SetContext(sessionID string, generation uint64) (Context, error) {
	if err := validateID("session_id", sessionID, metaSessionMax); err != nil {
		return Context{}, err
	}
	return s.registry.SetContext(sessionID, generation)
}

// InvalidateContext is hide/close invalidation: drop the context and
// abort everything.
func (s *Service) InvalidateContext() { s.registry.InvalidateContext() }

// Cancel is `speech_cancel` — idempotent, window-scoped.
func (s *Service) Cancel(requestID string) string { return s.registry.Cancel(requestID) }

func (s *Service) admitCommon(identity *Identity, kind RequestKind) (*CancelToken, time.Time, error) {
	if err := identity.Validate(); err != nil {
		return nil, time.Time{}, err
	}
	token, err := s.registry.Admit(identity.RequestID, identity.SessionID, identity.Generation, kind)
	if err != nil {
		return nil, time.Time{}, err
	}
	return token, time.Now(), nil
}

// prepare snapshots config + credential + (for inline references) the
// asset lease, OUTSIDE the registry lock. On failure the reservation is
// released so slots return to zero on every terminal path.
func (s *Service) prepare(identity *Identity, kind RequestKind, wav []byte, text string) (*prepared, error) {
	cleanup := func(err error) error {
		s.registry.Settle(identity.RequestID)
		return err
	}
	s.config.mu.Lock()
	prefs := s.config.v.Preferences()
	s.config.mu.Unlock()
	switch kind {
	case KindSTT:
		key, err := s.credentials.getSecret(prefs.STT.Provider)
		if err != nil {
			return nil, cleanup(err)
		}
		cfg := prefs.STT
		return &prepared{kind: kind, provider: cfg.Provider, sttCfg: &cfg, key: key, wav: wav}, nil
	default:
		switch prefs.TTS.Provider {
		case ProviderSiliconFlow:
			block := prefs.TTS.SiliconFlow
			if block == nil {
				return nil, cleanup(speechErr(CodeNotConfigured, "siliconflow TTS block not configured"))
			}
			cfg := *block
			key, err := s.credentials.getSecret(ProviderSiliconFlow)
			if err != nil {
				return nil, cleanup(err)
			}
			var inline *AssetLease
			if cfg.Reference != nil && !cfg.Reference.isSystem && cfg.Reference.VoiceID == "" {
				lease, err := s.assets.Read(cfg.Reference.AssetID)
				if err != nil {
					return nil, cleanup(err)
				}
				inline = lease
			}
			return &prepared{kind: kind, provider: ProviderSiliconFlow, sfCfg: &cfg, key: key, text: text, inline: inline}, nil
		case ProviderMiniMax:
			block := prefs.TTS.MiniMax
			if block == nil {
				return nil, cleanup(speechErr(CodeNotConfigured, "minimax TTS block not configured"))
			}
			cfg := *block
			key, err := s.credentials.getSecret(ProviderMiniMax)
			if err != nil {
				return nil, cleanup(err)
			}
			return &prepared{kind: kind, provider: ProviderMiniMax, mmCfg: &cfg, key: key, text: text}, nil
		default:
			return nil, cleanup(speechErr(CodeNotConfigured, "tts provider not configured"))
		}
	}
}

func (s *Service) configRevision() uint64 {
	s.config.mu.Lock()
	defer s.config.mu.Unlock()
	return s.config.v.Revision()
}

func (s *Service) diagnose(phase string, kind RequestKind, provider Provider, hasProv bool,
	started time.Time, bytes uint64, identity *Identity, err *SpeechError) {
	d := Diagnostic{
		Phase:          phase,
		Kind:           kind.String(),
		ElapsedMs:      uint64(time.Since(started).Milliseconds()),
		Bytes:          bytes,
		RequestID:      identity.RequestID,
		UtteranceID:    identity.UtteranceID,
		Generation:     identity.Generation,
		ConfigRevision: s.configRevision(),
	}
	if hasProv {
		d.Provider = provider.String()
	}
	if err != nil {
		d.Code = err.Code
		d.HTTPStatus = err.HTTPStatus
	}
	s.diagnostic(d)
}

func (s *Service) execute(p *prepared, token *CancelToken) (*preparedOutcome, error) {
	type result struct {
		out *preparedOutcome
		err error
	}
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	done := make(chan result, 1)
	go func() {
		defer func() {
			if p.inline != nil {
				p.inline.Release()
			}
		}()
		out, err := s.providerCall(ctx, p)
		done <- result{out, err}
	}()
	// A cancel arrives: abort the in-flight HTTP request too — the
	// request is truly torn down, not merely ignored.
	go func() {
		select {
		case <-token.Done():
			cancel()
		case <-ctx.Done():
		}
	}()
	select {
	case <-token.Done():
		return nil, speechErr(CodeCancelled, "request cancelled")
	case r := <-done:
		return r.out, r.err
	}
}

func (s *Service) providerCall(ctx context.Context, p *prepared) (*preparedOutcome, error) {
	switch {
	case p.sttCfg != nil:
		text, bytes, err := siliconflowSTT(ctx, s.client, p.sttCfg, p.key, p.wav, s.allowInsecureLoopback)
		if err != nil {
			return nil, err
		}
		return &preparedOutcome{isText: true, text: text, bytes: bytes, provider: ProviderSiliconFlow, hasProv: true}, nil
	case p.sfCfg != nil:
		mp3, bytes, err := siliconflowTTS(ctx, s.client, p.sfCfg, p.key, p.text, p.inline, s.allowInsecureLoopback)
		if err != nil {
			return nil, err
		}
		return &preparedOutcome{mp3: mp3, bytes: bytes, provider: ProviderSiliconFlow, hasProv: true}, nil
	default:
		mp3, bytes, err := minimaxTTS(ctx, s.client, p.mmCfg, p.key, p.text, s.allowInsecureLoopback)
		if err != nil {
			return nil, err
		}
		return &preparedOutcome{mp3: mp3, bytes: bytes, provider: ProviderMiniMax, hasProv: true}, nil
	}
}

// spawnRequest starts the request goroutine. The task owns provider
// I/O, the pre-settle hook, registry settle and diagnostics; the caller
// awaits the outcome channel.
func (s *Service) spawnRequest(kind RequestKind, identity *Identity, p *prepared,
	token *CancelToken, started time.Time) <-chan requestResult {
	outcomeCh := make(chan requestResult, 1)
	done := make(chan struct{})
	go func() {
		defer close(done)
		outcome, err := s.execute(p, token)
		s.preSettleHookMu.Lock()
		hook := s.preSettleHook
		s.preSettleHookMu.Unlock()
		if hook != nil {
			hook()
		}
		deliverable := s.registry.Settle(identity.RequestID)
		if err == nil && !deliverable {
			err = speechErr(CodeStaleContext,
				"request settled after its context advanced — result discarded")
			outcome = nil
		}
		var phase string
		var bytes uint64
		var provider Provider
		var hasProv bool
		var diagErr *SpeechError
		if err == nil {
			phase, bytes, provider, hasProv = "settled", outcome.bytes, outcome.provider, outcome.hasProv
		} else if se, ok := err.(*SpeechError); ok && se.Code == CodeCancelled {
			phase, provider, hasProv, diagErr = "cancelled", seProvider(se), se.Provider != "", se
		} else if se, ok := err.(*SpeechError); ok && se.Code == CodeStaleContext && !deliverable {
			phase, provider, hasProv, diagErr = "discarded", seProvider(se), se.Provider != "", se
		} else if se, ok := err.(*SpeechError); ok {
			phase, provider, hasProv, diagErr = "failed", seProvider(se), se.Provider != "", se
		} else {
			phase, diagErr = "failed", speechErr("internal", fmt.Sprintf("%v", err))
		}
		s.diagnose(phase, kind, provider, hasProv, started, bytes, identity, diagErr)
		outcomeCh <- requestResult{outcome, err}
	}()
	s.registry.Track(identity.RequestID, done)
	return outcomeCh
}

func seProvider(se *SpeechError) Provider {
	if se.Provider == "" {
		return ""
	}
	return Provider(se.Provider)
}

type requestResult struct {
	outcome *preparedOutcome
	err     error
}

// Transcribe is `speech_transcribe`: raw WAV body validated then
// uploaded.
func (s *Service) Transcribe(identity *Identity, wavBytes []byte) (*TranscribeReply, error) {
	if _, err := ValidateWAV(wavBytes); err != nil {
		return nil, err
	}
	token, started, err := s.admitCommon(identity, KindSTT)
	if err != nil {
		return nil, err
	}
	p, err := s.prepare(identity, KindSTT, wavBytes, "")
	if err != nil {
		return nil, err
	}
	s.diagnose("admitted", KindSTT, p.provider, true, started, 0, identity, nil)
	ch := s.spawnRequest(KindSTT, identity, p, token, started)
	res := <-ch
	if res.err != nil {
		return nil, res.err
	}
	text := res.outcome.text
	status := "transcribed"
	if trimEmpty(text) {
		status = "no_speech"
		text = ""
	}
	return &TranscribeReply{Identity: *identity, Status: status, Text: text}, nil
}

func trimEmpty(s string) bool {
	for _, r := range s {
		if r != ' ' && r != '\t' && r != '\n' && r != '\r' {
			return false
		}
	}
	return true
}

// Synthesize is `speech_synthesize`: JSON in, bounded MP3 bytes out.
func (s *Service) Synthesize(identity *Identity, text string) ([]byte, error) {
	if text == "" || utf8.RuneCountInString(text) > TTSTextMaxChars || len(text) > TTSTextMaxBytes {
		return nil, speechErr(CodeInvalidInput, "text empty or exceeds TTS bounds")
	}
	token, started, err := s.admitCommon(identity, KindTTS)
	if err != nil {
		return nil, err
	}
	p, err := s.prepare(identity, KindTTS, nil, text)
	if err != nil {
		return nil, err
	}
	s.diagnose("admitted", KindTTS, p.provider, true, started, 0, identity, nil)
	ch := s.spawnRequest(KindTTS, identity, p, token, started)
	res := <-ch
	if res.err != nil {
		return nil, res.err
	}
	return res.outcome.mp3, nil
}

// Shutdown rejects admission, aborts in-flight requests and joins their
// tasks inside grace. The report states honestly what could not be
// joined.
func (s *Service) Shutdown(grace time.Duration) ShutdownReport {
	ctx, cancel := context.WithTimeout(context.Background(), grace)
	defer cancel()
	return s.ShutdownContext(ctx)
}

// ShutdownContext rejects admission, aborts in-flight requests, and joins
// their tasks until the caller's shared shutdown deadline. It does not
// create a fresh grace period of its own.
func (s *Service) ShutdownContext(ctx context.Context) ShutdownReport {
	s.registry.BeginShutdown()
	inflight, handles := s.registry.TakeHandles()
	joined := 0
	for _, h := range handles {
		select {
		case <-h:
			joined++
		case <-ctx.Done():
			return ShutdownReport{InflightAtStart: inflight, Joined: joined, Remaining: len(handles) - joined}
		}
	}
	// Remaining tasks were already aborted via BeginShutdown tokens;
	// the report states honestly what could not be joined.
	return ShutdownReport{InflightAtStart: inflight, Joined: joined, Remaining: len(handles) - joined}
}
