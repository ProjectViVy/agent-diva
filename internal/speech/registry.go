package speech

// Admission registry: trusted window context (session + monotonic
// generation), one STT and one TTS slot, cancel tokens, and owned
// done-channel handles for abort/join teardown. The registry mutex is
// never held across keyring or HTTP work.

import (
	"sync"
	"sync/atomic"
)

// RequestKind selects the single-flight slot.
type RequestKind int

const (
	KindSTT RequestKind = iota
	KindTTS
)

func (k RequestKind) String() string {
	if k == KindSTT {
		return "stt"
	}
	return "tts"
}

// Context is the trusted window context: the session the speech lane is
// bound to and the caller's monotonic generation counter.
type Context struct {
	SessionID  string `json:"session_id"`
	Generation uint64 `json:"generation"`
}

// CancelToken is a cross-task abort signal: closed channel + flag, so a
// waiter created after the cancel still observes it.
type CancelToken struct {
	flag atomic.Bool
	ch   chan struct{}
	once sync.Once
}

func NewCancelToken() *CancelToken {
	return &CancelToken{ch: make(chan struct{})}
}

func (t *CancelToken) Cancel() {
	t.once.Do(func() {
		t.flag.Store(true)
		close(t.ch)
	})
}

func (t *CancelToken) IsCancelled() bool { return t.flag.Load() }

// Done resolves once cancelled — closed channel is permanently
// readable, no check-then-wait hole.
func (t *CancelToken) Done() <-chan struct{} { return t.ch }

type inflight struct {
	sessionID  string
	generation uint64
	token      *CancelToken
}

type registryInner struct {
	context    *Context
	quitting   bool
	sttHolder  string
	ttsHolder  string
	inflight   map[string]*inflight
	handles    map[string]chan struct{} // request_id -> done
}

// Registry is the admission registry.
type Registry struct {
	mu sync.Mutex
	in registryInner
}

func NewRegistry() *Registry {
	return &Registry{in: registryInner{
		inflight: map[string]*inflight{},
		handles:  map[string]chan struct{}{},
	}}
}

func busyPoison() *SpeechError {
	return speechErr(CodeBusy, "speech registry lock poisoned")
}

// SetContext: identical tuple is idempotent; a strictly newer generation
// accepts and aborts every in-flight request of the old context; an
// older or conflicting tuple is rejected.
func (r *Registry) SetContext(sessionID string, generation uint64) (Context, error) {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.in.quitting {
		return Context{}, speechErr(CodeCancelled, "speech lane is shutting down")
	}
	if cur := r.in.context; cur != nil {
		if cur.SessionID == sessionID && cur.Generation == generation {
			return *cur, nil
		}
		if generation <= cur.Generation {
			return Context{}, speechErr(CodeStaleContext,
				"context tuple is older than or conflicts with the current context")
		}
	}
	next := Context{SessionID: sessionID, Generation: generation}
	r.in.context = &next
	r.abortAllLocked()
	return next, nil
}

func (r *Registry) CurrentContext() *Context {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.in.context == nil {
		return nil
	}
	c := *r.in.context
	return &c
}

// Admit validates identity against the trusted context, reserves
// request_id and the kind's single slot, and returns the request's
// cancel token. The lock is released before the caller does
// keyring/HTTP work.
func (r *Registry) Admit(requestID, sessionID string, generation uint64, kind RequestKind) (*CancelToken, error) {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.in.quitting {
		return nil, speechErr(CodeCancelled, "speech lane is shutting down")
	}
	ctx := r.in.context
	if ctx == nil {
		return nil, speechErr(CodeNotConfigured, "no speech context set")
	}
	if ctx.SessionID != sessionID || ctx.Generation != generation {
		return nil, speechErr(CodeStaleContext,
			"request identity does not match the current context")
	}
	if _, ok := r.in.inflight[requestID]; ok {
		return nil, speechErr(CodeInvalidInput, "request_id already admitted")
	}
	if kind == KindSTT && r.in.sttHolder != "" {
		return nil, speechErr(CodeBusy, "a stt request is already in flight").retryable(true)
	}
	if kind == KindTTS && r.in.ttsHolder != "" {
		return nil, speechErr(CodeBusy, "a tts request is already in flight").retryable(true)
	}
	if kind == KindSTT {
		r.in.sttHolder = requestID
	} else {
		r.in.ttsHolder = requestID
	}
	token := NewCancelToken()
	r.in.inflight[requestID] = &inflight{
		sessionID: sessionID, generation: generation, token: token,
	}
	return token, nil
}

// Settle frees the request's slot and reports whether the outcome is
// still deliverable (context unchanged, not cancelled, not quitting).
// A non-deliverable success MUST be discarded.
func (r *Registry) Settle(requestID string) bool {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.reapLocked()
	entry, ok := r.in.inflight[requestID]
	if !ok {
		return false
	}
	delete(r.in.inflight, requestID)
	if r.in.sttHolder == requestID {
		r.in.sttHolder = ""
	}
	if r.in.ttsHolder == requestID {
		r.in.ttsHolder = ""
	}
	if r.in.quitting || entry.token.IsCancelled() {
		return false
	}
	ctx := r.in.context
	return ctx != nil && ctx.SessionID == entry.sessionID && ctx.Generation == entry.generation
}

// Cancel is window-scoped and idempotent. Returns "cancelled" when an
// in-flight request was aborted, else "settled" — local abort never
// claims a remote refund.
func (r *Registry) Cancel(requestID string) string {
	r.mu.Lock()
	defer r.mu.Unlock()
	if entry, ok := r.in.inflight[requestID]; ok {
		entry.token.Cancel()
		return "cancelled"
	}
	return "settled"
}

// InvalidateContext drops the context and aborts everything in flight
// (window hide).
func (r *Registry) InvalidateContext() {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.in.context = nil
	r.abortAllLocked()
}

// BeginShutdown rejects future admission and aborts in-flight requests.
func (r *Registry) BeginShutdown() {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.in.quitting = true
	r.in.context = nil
	r.abortAllLocked()
}

// Track registers a spawned task done-channel for quit-time joining.
func (r *Registry) Track(requestID string, done chan struct{}) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.reapLocked()
	r.in.handles[requestID] = done
}

// TakeHandles detaches every tracked handle so the caller can join them
// outside the lock. Returns (in-flight request count, handles).
func (r *Registry) TakeHandles() (int, []chan struct{}) {
	r.mu.Lock()
	defer r.mu.Unlock()
	out := make([]chan struct{}, 0, len(r.in.handles))
	for _, h := range r.in.handles {
		out = append(out, h)
	}
	r.in.handles = map[string]chan struct{}{}
	return len(r.in.inflight), out
}

// reapLocked drops finished handles.
func (r *Registry) reapLocked() {
	for id, done := range r.in.handles {
		select {
		case <-done:
			delete(r.in.handles, id)
		default:
		}
	}
}

// ActiveCount is the test/diagnostic count of live slots and requests.
func (r *Registry) ActiveCount() int {
	r.mu.Lock()
	defer r.mu.Unlock()
	return len(r.in.inflight)
}

func (r *Registry) IsQuitting() bool {
	r.mu.Lock()
	defer r.mu.Unlock()
	return r.in.quitting
}

func (r *Registry) abortAllLocked() {
	for _, entry := range r.in.inflight {
		entry.token.Cancel()
	}
	r.in.sttHolder = ""
	r.in.ttsHolder = ""
}
