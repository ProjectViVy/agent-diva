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
	id   uint
	name string
}

func (f fakeWindow) ID() uint     { return f.id }
func (f fakeWindow) Name() string { return f.name }

func applicationServiceOptions() application.ServiceOptions {
	return application.ServiceOptions{}
}

type fakeHost struct {
	callFn      func(method string, params json.RawMessage) (json.RawMessage, error)
	callContext func(context.Context)
	calls       int
	batches     []hostv1.EventBatch
	nextErr     error
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
	defer f.mu.Unlock()
	if len(f.batches) > 0 {
		b := f.batches[0]
		f.batches = f.batches[1:]
		return b, nil
	}
	if f.nextErr != nil {
		return hostv1.EventBatch{}, f.nextErr
	}
	<-ctx.Done()
	return hostv1.EventBatch{}, ctx.Err()
}

func (f *fakeHost) Close(_ context.Context) error {
	f.mu.Lock()
	defer f.mu.Unlock()
	f.closed = true
	return nil
}

func (f *fakeHost) isClosed() bool {
	f.mu.Lock()
	defer f.mu.Unlock()
	return f.closed
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
	svc.stopPump()
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
