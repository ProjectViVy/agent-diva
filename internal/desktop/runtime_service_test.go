package desktop

import (
	"context"
	"encoding/json"
	"errors"
	"sync"
	"testing"
	"time"

	hostv1 "agent-vivy/sdk/host/v1"
	"github.com/wailsapp/wails/v3/pkg/application"
)

// windowCtxKey is the binding-context key wails injects natively.
var windowCtxKey = application.WindowKey

type fakeWindow struct {
	application.Window
	id         uint
	name       string
	showCalls  *int
	focusCalls *int
}

func (f fakeWindow) ID() uint     { return f.id }
func (f fakeWindow) Name() string { return f.name }
func (f fakeWindow) Show() application.Window {
	if f.showCalls != nil {
		(*f.showCalls)++
	}
	return f
}
func (f fakeWindow) Focus() {
	if f.focusCalls != nil {
		(*f.focusCalls)++
	}
}

func applicationServiceOptions() application.ServiceOptions {
	return application.ServiceOptions{}
}

type fakeHost struct {
	callFn      func(method string, params json.RawMessage) (json.RawMessage, error)
	callContext func(context.Context)
	calls       int
	batches     []hostv1.EventBatch
	nextErr     error
	nextFn      func(context.Context, int) (hostv1.EventBatch, error)
	closeFn     func(context.Context) error
	closeCalls  int
	closed      bool
	mu          sync.Mutex
}

func (f *fakeHost) Call(ctx context.Context, method string, params json.RawMessage) (json.RawMessage, error) {
	f.mu.Lock()
	f.calls++
	f.mu.Unlock()
	if f.callContext != nil {
		f.callContext(ctx)
	}
	if f.callFn != nil {
		return f.callFn(method, params)
	}
	return json.RawMessage(`{"ok":true}`), nil
}

func (f *fakeHost) callCount() int {
	f.mu.Lock()
	defer f.mu.Unlock()
	return f.calls
}

func (f *fakeHost) Next(ctx context.Context, _ int) (hostv1.EventBatch, error) {
	f.mu.Lock()
	nextFn := f.nextFn
	if nextFn != nil {
		f.mu.Unlock()
		return nextFn(ctx, nextBatchLimit)
	}
	if len(f.batches) > 0 {
		b := f.batches[0]
		f.batches = f.batches[1:]
		f.mu.Unlock()
		return b, nil
	}
	nextErr := f.nextErr
	f.mu.Unlock()
	if nextErr != nil {
		return hostv1.EventBatch{}, nextErr
	}
	<-ctx.Done()
	return hostv1.EventBatch{}, ctx.Err()
}

func (f *fakeHost) Close(ctx context.Context) error {
	f.mu.Lock()
	f.closeCalls++
	closeFn := f.closeFn
	f.mu.Unlock()
	if closeFn != nil {
		if err := closeFn(ctx); err != nil {
			return err
		}
	}
	f.mu.Lock()
	f.closed = true
	f.mu.Unlock()
	return nil
}

func (f *fakeHost) isClosed() bool {
	f.mu.Lock()
	defer f.mu.Unlock()
	return f.closed
}

func (f *fakeHost) closeCallCount() int {
	f.mu.Lock()
	defer f.mu.Unlock()
	return f.closeCalls
}

type emitSink struct {
	mu     sync.Mutex
	events []wireEvent
}

func (s *emitSink) emit(_ string, data any) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if ev, ok := data.(wireEvent); ok {
		s.events = append(s.events, ev)
	}
}

func (s *emitSink) snapshot() []wireEvent {
	s.mu.Lock()
	defer s.mu.Unlock()
	return append([]wireEvent(nil), s.events...)
}

func TestVivyCallEnvelope(t *testing.T) {
	cap := newMediaCapability(0, "tok")
	cap.bind(42)
	var hasDeadline bool
	h := &fakeHost{callFn: func(method string, params json.RawMessage) (json.RawMessage, error) {
		if method == "boom" {
			return nil, &hostv1.Error{Kind: "rpc", Code: -32000, Message: "upstream"}
		}
		return json.RawMessage(`{"echo":` + string(params) + `}`), nil
	}, callContext: func(ctx context.Context) {
		_, hasDeadline = ctx.Deadline()
	}}
	svc := NewRuntimeService(h, func(string, any) {})
	svc.cap = cap
	main := context.WithValue(context.Background(), windowCtxKey, fakeWindow{id: 42})

	ok := svc.VivyCall(main, CallRequest{Method: "m", Params: json.RawMessage(`{"a":1}`), TimeoutMs: 500})
	if !ok.OK || string(ok.Result) != `{"echo":{"a":1}}` {
		t.Fatalf("want ok reply, got %+v", ok)
	}
	if !hasDeadline {
		t.Fatal("requested timeout was not applied to host call")
	}
	bad := svc.VivyCall(main, CallRequest{Method: "boom"})
	if bad.OK || bad.Error == nil || bad.Error.Kind != "rpc" || bad.Error.Code != -32000 {
		t.Fatalf("want preserved rpc error, got %+v", bad.Error)
	}
	empty := svc.VivyCall(main, CallRequest{})
	if empty.OK || empty.Error == nil || empty.Error.Kind != "invalid_input" || empty.Error.Code != -32602 {
		t.Fatalf("want invalid_input for empty method, got %+v", empty.Error)
	}
}

func TestVivyCallNativeAuthorization(t *testing.T) {
	tests := []struct {
		name string
		ctx  context.Context
		cap  *mediaCapability
	}{
		{name: "missing identity", ctx: context.Background(), cap: newMediaCapability(42, "tok")},
		{name: "foreign window", ctx: context.WithValue(context.Background(), windowCtxKey, fakeWindow{id: 7}), cap: newMediaCapability(42, "tok")},
		{name: "nil capability", ctx: context.WithValue(context.Background(), windowCtxKey, fakeWindow{id: 42})},
		{name: "unbound capability", ctx: context.WithValue(context.Background(), windowCtxKey, fakeWindow{id: 42}), cap: newMediaCapability(0, "tok")},
	}
	revoked := newMediaCapability(42, "tok")
	revoked.revoke()
	tests = append(tests, struct {
		name string
		ctx  context.Context
		cap  *mediaCapability
	}{name: "revoked capability", ctx: context.WithValue(context.Background(), windowCtxKey, fakeWindow{id: 42}), cap: revoked})

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			h := &fakeHost{}
			svc := NewRuntimeService(h, func(string, any) {})
			svc.cap = tt.cap
			dispatchCalls := 0
			svc.dispatch = map[string]func(context.Context, json.RawMessage) (json.RawMessage, error){
				"privileged": func(context.Context, json.RawMessage) (json.RawMessage, error) {
					dispatchCalls++
					return json.RawMessage(`{}`), nil
				},
			}
			reply := svc.VivyCall(tt.ctx, CallRequest{Method: "privileged.call"})
			if reply.OK || reply.Error == nil || reply.Error.Kind != "closed" || reply.Error.Code != -32081 {
				t.Fatalf("unauthorized call reply = %+v", reply)
			}
			if got := h.callCount(); got != 0 {
				t.Fatalf("unauthorized call reached host %d times", got)
			}
			dispatched := svc.DesktopDispatch(tt.ctx, DispatchRequest{Command: "privileged"})
			if dispatched.OK || dispatched.Error == nil || dispatched.Error.Kind != "closed" || dispatched.Error.Code != -32081 {
				t.Fatalf("unauthorized dispatch reply = %+v", dispatched)
			}
			if dispatchCalls != 0 {
				t.Fatalf("unauthorized dispatch reached handler %d times", dispatchCalls)
			}
			if token, err := svc.MediaToken(tt.ctx); token != "" || err == nil {
				t.Fatalf("unauthorized identity received token=%q err=%v", token, err)
			}
		})
	}
}

func TestPrivilegedMethodsRejectRevokedCapability(t *testing.T) {
	cap := newMediaCapability(42, "tok")
	cap.revoke()
	ctx := context.WithValue(context.Background(), windowCtxKey, fakeWindow{id: 42})
	h := &fakeHost{}
	svc := NewRuntimeService(h, func(string, any) {})
	svc.cap = cap
	dispatchCalls := 0
	svc.dispatch = map[string]func(context.Context, json.RawMessage) (json.RawMessage, error){
		"privileged": func(context.Context, json.RawMessage) (json.RawMessage, error) {
			dispatchCalls++
			return json.RawMessage(`{}`), nil
		},
	}

	rpc := svc.VivyCall(ctx, CallRequest{Method: "privileged.call"})
	if rpc.OK || rpc.Error == nil || rpc.Error.Kind != "closed" || rpc.Error.Code != -32081 || h.callCount() != 0 {
		t.Fatalf("revoked RPC capability was not denied before host call: reply=%+v calls=%d", rpc, h.callCount())
	}
	dispatched := svc.DesktopDispatch(ctx, DispatchRequest{Command: "privileged"})
	if dispatched.OK || dispatched.Error == nil || dispatched.Error.Kind != "closed" || dispatched.Error.Code != -32081 || dispatchCalls != 0 {
		t.Fatalf("revoked dispatch capability was not denied before handler: reply=%+v handlerCalls=%d", dispatched, dispatchCalls)
	}
	if token, err := svc.MediaToken(ctx); token != "" || err == nil {
		t.Fatalf("revoked capability returned token=%q err=%v", token, err)
	}
}

func TestAuthorizationPrecedesPayloadValidation(t *testing.T) {
	h := &fakeHost{}
	svc := NewRuntimeService(h, func(string, any) {})
	denied := svc.VivyCall(context.Background(), CallRequest{})
	if denied.OK || denied.Error == nil || denied.Error.Kind != "closed" || denied.Error.Code != -32081 {
		t.Fatalf("unauthorized empty method disclosed validation result: %+v", denied)
	}
	if got := h.callCount(); got != 0 {
		t.Fatalf("unauthorized empty method reached host %d times", got)
	}

	cap := newMediaCapability(42, "tok")
	cap.bind(42)
	svc.cap = cap
	main := context.WithValue(context.Background(), windowCtxKey, fakeWindow{id: 42})
	validated := svc.VivyCall(main, CallRequest{})
	if validated.OK || validated.Error == nil || validated.Error.Kind != "invalid_input" || validated.Error.Code != -32602 {
		t.Fatalf("authorized empty method should retain invalid-input envelope: %+v", validated)
	}
	if got := h.callCount(); got != 0 {
		t.Fatalf("empty method reached host %d times", got)
	}
}

func TestPumpForwardsEventsGapAndLoss(t *testing.T) {
	boom := errors.New("transport gone")
	h := &fakeHost{
		batches: []hostv1.EventBatch{
			{Events: []hostv1.Notification{{Method: "run.delta", Params: json.RawMessage(`{"x":1}`)}}, Gap: true},
		},
		nextErr: boom,
	}
	sink := &emitSink{}
	svc := NewRuntimeService(h, sink.emit)
	if err := svc.ServiceStartup(context.Background(), applicationServiceOptions()); err != nil {
		t.Fatal(err)
	}
	deadline := time.Now().Add(2 * time.Second)
	for time.Now().Before(deadline) {
		evs := sink.snapshot()
		if len(evs) >= 3 {
			if evs[0].Kind != "bridge" || evs[0].Status != "gap" {
				t.Fatalf("want gap first, got %+v", evs)
			}
			if evs[1].Kind != "vivy" || evs[1].Method != "run.delta" {
				t.Fatalf("want vivy event, got %+v", evs)
			}
			if evs[2].Kind != "bridge" || evs[2].Status != "lost" {
				t.Fatalf("want lost last, got %+v", evs)
			}
			break
		}
		time.Sleep(10 * time.Millisecond)
	}
	if len(sink.snapshot()) < 3 {
		t.Fatalf("pump emitted %d events, want >=3", len(sink.snapshot()))
	}
	_ = svc.stopPump(context.Background())
}

func TestServiceShutdownStopsPumpAndClosesHost(t *testing.T) {
	h := &fakeHost{}
	var recorded error
	svc := NewRuntimeService(h, func(string, any) {})
	svc.onShutdownErr = func(err error) { recorded = err }
	if err := svc.ServiceStartup(context.Background(), applicationServiceOptions()); err != nil {
		t.Fatal(err)
	}
	if err := svc.ServiceShutdown(); err != nil {
		t.Fatal(err)
	}
	if !h.isClosed() {
		t.Fatal("host not closed")
	}
	if recorded != nil {
		t.Fatalf("teardown error recorded: %v", recorded)
	}
	// Idempotent second call must not hang on pumpDone.
	if err := svc.ServiceShutdown(); err != nil {
		t.Fatal(err)
	}
}

func TestShutdownBudgetStartsBeforePumpJoin(t *testing.T) {
	entered := make(chan struct{})
	cancelled := make(chan struct{})
	release := make(chan struct{})
	deadlineSeen := make(chan time.Time, 1)
	h := &fakeHost{
		nextFn: func(ctx context.Context, _ int) (hostv1.EventBatch, error) {
			close(entered)
			<-ctx.Done()
			close(cancelled)
			<-release // model a producer which ignores cancellation
			return hostv1.EventBatch{}, ctx.Err()
		},
		closeFn: func(ctx context.Context) error {
			deadline, ok := ctx.Deadline()
			if !ok {
				t.Error("host close did not receive the shared deadline")
				return nil
			}
			deadlineSeen <- deadline
			return nil
		},
	}
	svc := NewRuntimeService(h, func(string, any) {})
	if err := svc.ServiceStartup(context.Background(), applicationServiceOptions()); err != nil {
		t.Fatal(err)
	}
	<-entered
	started := make(chan time.Time, 1)
	shutdown := make(chan error, 1)
	go func() {
		started <- time.Now()
		shutdown <- svc.ServiceShutdown()
	}()
	start := <-started
	<-cancelled
	timer := time.NewTimer(75 * time.Millisecond)
	<-timer.C
	close(release)
	if err := <-shutdown; err != nil {
		t.Fatal(err)
	}
	deadline := <-deadlineSeen
	if deadline.After(start.Add(hostv1.CloseBudget + 25*time.Millisecond)) {
		t.Fatalf("host received a reset shutdown budget: start=%s deadline=%s", start, deadline)
	}
}

func TestShutdownPreservesSpeechAndHostErrors(t *testing.T) {
	speechErr := errors.New("speech drain incomplete")
	hostErr := errors.New("host close incomplete")
	h := &fakeHost{closeFn: func(context.Context) error { return hostErr }}
	l := newLifecycle(func(string, any) {})
	svc := NewRuntimeService(h, func(string, any) {})
	svc.onShutdown = func(context.Context) error { return speechErr }
	svc.onShutdownErr = l.recordTeardown
	err := svc.ServiceShutdown()
	if !errors.Is(err, speechErr) || !errors.Is(err, hostErr) {
		t.Fatalf("shutdown error = %v, want speech and host errors", err)
	}
}

func TestShutdownSharesDeadlineWithSpeechAndHost(t *testing.T) {
	var speechDeadline, hostDeadline time.Time
	h := &fakeHost{closeFn: func(ctx context.Context) error {
		hostDeadline, _ = ctx.Deadline()
		return nil
	}}
	svc := NewRuntimeService(h, func(string, any) {})
	svc.onShutdown = func(ctx context.Context) error {
		speechDeadline, _ = ctx.Deadline()
		return nil
	}
	ctx, cancel := context.WithTimeout(context.Background(), time.Second)
	defer cancel()
	wantDeadline, _ := ctx.Deadline()
	if err := svc.shutdown(ctx); err != nil {
		t.Fatal(err)
	}
	if !speechDeadline.Equal(wantDeadline) || !hostDeadline.Equal(wantDeadline) {
		t.Fatalf("shutdown deadlines differ: want=%s speech=%s host=%s", wantDeadline, speechDeadline, hostDeadline)
	}
}

func TestShutdownDoesNotReportCleanOrReopenWhileDrainPending(t *testing.T) {
	entered := make(chan struct{})
	release := make(chan struct{})
	closeStarted := make(chan struct{}, 1)
	h := &fakeHost{
		nextFn: func(ctx context.Context, _ int) (hostv1.EventBatch, error) {
			close(entered)
			<-ctx.Done()
			<-release
			return hostv1.EventBatch{}, ctx.Err()
		},
		closeFn: func(context.Context) error {
			closeStarted <- struct{}{}
			return nil
		},
	}
	l := newLifecycle(func(string, any) {})
	showCalls, focusCalls := 0, 0
	w := &fakeWindow{id: 42, name: "main", showCalls: &showCalls, focusCalls: &focusCalls}
	var shown int
	d := &Desktop{window: w, lifecycle: l, emit: func(string, any) { shown++ }}
	cap := newMediaCapability(42, "token")
	svc := NewRuntimeService(h, func(string, any) {})
	svc.cap = cap
	svc.closeAdmission = l.closeAdmission
	svc.onShutdownStart = l.beginTeardown
	svc.onShutdownErr = l.recordTeardown
	if err := svc.ServiceStartup(context.Background(), applicationServiceOptions()); err != nil {
		t.Fatal(err)
	}
	<-entered
	ctx, cancel := context.WithTimeout(context.Background(), 50*time.Millisecond)
	defer cancel()
	err := svc.shutdown(ctx)
	if !errors.Is(err, context.DeadlineExceeded) {
		t.Fatalf("bounded shutdown error = %v, want deadline", err)
	}
	if _, ok := cap.tokenFor(42); ok {
		t.Fatal("capability remained valid after shutdown admission closed")
	}
	d.lifecycle.requestReopen(d.reopen)
	if showCalls != 0 || focusCalls != 0 || shown != 0 {
		t.Fatalf("pending drain reopened the window: show=%d focus=%d events=%d", showCalls, focusCalls, shown)
	}
	if finished, teardownErr := l.teardownState(); finished || teardownErr != nil {
		t.Fatalf("pending drain was reported complete: finished=%t err=%v", finished, teardownErr)
	}
	select {
	case <-closeStarted:
		t.Fatal("host closed before the blocked pump drained")
	default:
	}
	close(release)
	if err := svc.shutdown(context.Background()); !errors.Is(err, context.DeadlineExceeded) {
		t.Fatalf("retained shutdown result = %v, want deadline", err)
	}
	if got := h.closeCallCount(); got != 1 {
		t.Fatalf("host close calls after drain = %d, want one", got)
	}
	if finished, teardownErr := l.teardownState(); !finished || !errors.Is(teardownErr, context.DeadlineExceeded) {
		t.Fatalf("completed drain state = (%t, %v), want retained timeout", finished, teardownErr)
	}
}

func TestShutdownIdempotentOutcome(t *testing.T) {
	hostErr := errors.New("close failed")
	h := &fakeHost{closeFn: func(context.Context) error { return hostErr }}
	svc := NewRuntimeService(h, func(string, any) {})
	first := svc.ServiceShutdown()
	second := svc.ServiceShutdown()
	if !errors.Is(first, hostErr) || !errors.Is(second, hostErr) {
		t.Fatalf("shutdown outcomes differ: first=%v second=%v", first, second)
	}
	if got := h.closeCallCount(); got != 1 {
		t.Fatalf("host Close called %d times, want once", got)
	}
}

func TestPumpCallbackFailureRetained(t *testing.T) {
	boom := errors.New("event consumer panicked")
	h := &fakeHost{batches: []hostv1.EventBatch{{Events: []hostv1.Notification{{Method: "run.delta"}}}}}
	svc := NewRuntimeService(h, func(string, any) { panic(boom) })
	svc.pumpDone = make(chan struct{})
	var panicValue any
	func() {
		defer func() { panicValue = recover() }()
		svc.pump(context.Background())
	}()
	if panicValue != nil {
		t.Fatalf("pump allowed emitter panic to escape: %v", panicValue)
	}
	if err := svc.ServiceShutdown(); !errors.Is(err, boom) {
		t.Fatalf("shutdown error = %v, want retained emitter panic", err)
	}
}

func TestPumpFailureAfterCallerDeadlineIsRetained(t *testing.T) {
	boom := errors.New("late emitter panic")
	emitterEntered := make(chan struct{})
	releaseEmitter := make(chan struct{})
	h := &fakeHost{batches: []hostv1.EventBatch{{Events: []hostv1.Notification{{Method: "run.delta"}}}}}
	svc := NewRuntimeService(h, func(string, any) {
		close(emitterEntered)
		<-releaseEmitter
		panic(boom)
	})
	if err := svc.ServiceStartup(context.Background(), applicationServiceOptions()); err != nil {
		t.Fatal(err)
	}
	<-emitterEntered
	ctx, cancel := context.WithTimeout(context.Background(), 50*time.Millisecond)
	defer cancel()
	if err := svc.shutdown(ctx); !errors.Is(err, context.DeadlineExceeded) {
		t.Fatalf("initial shutdown error = %v, want deadline", err)
	}
	close(releaseEmitter)
	err := svc.shutdown(context.Background())
	if !errors.Is(err, boom) {
		t.Fatalf("retained shutdown error = %v, want late pump failure", err)
	}
}

func TestShutdownRevokesCapability(t *testing.T) {
	cap := newMediaCapability(42, "tok")
	svc := NewRuntimeService(&fakeHost{}, func(string, any) {})
	svc.cap = cap
	if err := svc.ServiceShutdown(); err != nil {
		t.Fatal(err)
	}
	if _, ok := cap.tokenFor(42); ok {
		t.Fatal("shutdown retained the native window capability")
	}
}

func TestDesktopDispatchGate(t *testing.T) {
	cap := newMediaCapability(0, "tok")
	cap.bind(42)
	svc := NewRuntimeService(&fakeHost{}, func(string, any) {})
	svc.cap = cap

	main := context.WithValue(context.Background(), windowCtxKey, fakeWindow{id: 42})
	// Unimplemented commands answer not_ready — never a silent noop.
	r := svc.DesktopDispatch(main, DispatchRequest{Command: "speech_config_get"})
	if r.OK || r.Error == nil || r.Error.Kind != "not_ready" {
		t.Fatalf("want not_ready, got %+v", r)
	}
	// No window identity and foreign windows are both denied.
	if r := svc.DesktopDispatch(context.Background(), DispatchRequest{Command: "x"}); r.OK {
		t.Fatal("no-identity dispatch allowed")
	}
	foreign := context.WithValue(context.Background(), windowCtxKey, fakeWindow{id: 7})
	if r := svc.DesktopDispatch(foreign, DispatchRequest{Command: "x"}); r.OK {
		t.Fatal("foreign-window dispatch allowed")
	}
	// A registered handler runs and preserves result/error shape.
	svc.dispatch = map[string]func(context.Context, json.RawMessage) (json.RawMessage, error){
		"echo": func(_ context.Context, p json.RawMessage) (json.RawMessage, error) { return p, nil },
		"boom": func(context.Context, json.RawMessage) (json.RawMessage, error) {
			return nil, &hostv1.Error{Kind: "timeout", Code: -32083, Message: "slow"}
		},
	}
	r = svc.DesktopDispatch(main, DispatchRequest{Command: "echo", Payload: json.RawMessage(`{"a":1}`)})
	if !r.OK || string(r.Result) != `{"a":1}` {
		t.Fatalf("want echo result, got %+v", r)
	}
	r = svc.DesktopDispatch(main, DispatchRequest{Command: "boom"})
	if r.OK || r.Error == nil || r.Error.Kind != "timeout" {
		t.Fatalf("want preserved timeout, got %+v", r)
	}
}

func TestMediaTokenBoundToMainWindow(t *testing.T) {
	cap := newMediaCapability(0, "tok")
	cap.bind(42)
	svc := NewRuntimeService(&fakeHost{}, func(string, any) {})
	svc.cap = cap

	ctx := context.WithValue(context.Background(), windowCtxKey, fakeWindow{id: 42})
	tok, err := svc.MediaToken(ctx)
	if err != nil || tok != "tok" {
		t.Fatalf("main window: tok=%q err=%v", tok, err)
	}
	foreign := context.WithValue(context.Background(), windowCtxKey, fakeWindow{id: 7})
	if _, err := svc.MediaToken(foreign); err == nil {
		t.Fatal("foreign window got token")
	}
	if _, err := svc.MediaToken(context.Background()); err == nil {
		t.Fatal("no-identity ctx got token")
	}
}
