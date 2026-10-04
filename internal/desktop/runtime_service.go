package desktop

import (
	"context"
	"encoding/json"
	"errors"
	"sync"
	"time"

	hostv1 "agent-vivy/sdk/host/v1"
	"github.com/wailsapp/wails/v3/pkg/application"
)

// nextBatchLimit bounds one Next drain (W3-2 event pump).
const nextBatchLimit = 64

// CallRequest is the bound vivy_call request; the shell never interprets
// business method names.
type CallRequest struct {
	Method    string          `json:"method"`
	Params    json.RawMessage `json:"params,omitempty"`
	TimeoutMs int             `json:"timeoutMs,omitempty"`
}

// CallReply is the normalized answer envelope. host.Error travels as data
// (kind/code/message/data) because a bound-method Go error would be flattened
// to a string by the binding layer.
type CallReply struct {
	OK     bool            `json:"ok"`
	Result json.RawMessage `json:"result,omitempty"`
	Error  *hostv1.Error   `json:"error,omitempty"`
}

// wireEvent mirrors the frontend WireEvent union on vivy:event.
type wireEvent struct {
	Kind   string          `json:"kind"` // "vivy" | "bridge"
	Method string          `json:"method,omitempty"`
	Params json.RawMessage `json:"params,omitempty"`
	Status string          `json:"status,omitempty"` // "gap" | "lost"
}

const eventChannel = "vivy:event"

// RuntimeService owns the single VIVY host for the desktop: bound calls,
// the blocking-Next event pump, and the bounded teardown. Registered as a
// Wails service — ServiceShutdown runs after OnShutdown and before
// PostShutdown (W0-recorded order).
// hostAPI is the host surface the service uses; *hostv1.Host satisfies it
// and tests substitute a fake.
type hostAPI interface {
	Call(ctx context.Context, method string, params json.RawMessage) (json.RawMessage, error)
	Next(ctx context.Context, limit int) (hostv1.EventBatch, error)
	Close(ctx context.Context) error
}

type RuntimeService struct {
	host hostAPI
	emit func(name string, data any)

	// onShutdownErr records the coordinator outcome for PostShutdown;
	// set by the desktop during Compose.
	onShutdownErr func(error)

	// cap issues the media token only to the bound main window (W3-3/W3-5).
	cap *mediaCapability

	// dispatch maps native command names to handlers; nil until W4 wires
	// the speech/credential/asset table.
	dispatch map[string]func(ctx context.Context, payload json.RawMessage) (json.RawMessage, error)

	pumpCancel context.CancelFunc
	pumpDone   chan struct{}
	pumpOnce   sync.Once
}

// NewRuntimeService wires a host to an event emitter (app.Event.Emit or a
// test sink).
func NewRuntimeService(host hostAPI, emit func(name string, data any)) *RuntimeService {
	return &RuntimeService{host: host, emit: emit}
}

func (s *RuntimeService) ServiceStartup(_ context.Context, _ application.ServiceOptions) error {
	ctx, cancel := context.WithCancel(context.Background())
	s.pumpCancel = cancel
	s.pumpDone = make(chan struct{})
	go s.pump(ctx)
	return nil
}

// ServiceShutdown is the single teardown coordinator: stop the producer
// (pump), then close the host under one deadline and report honestly.
func (s *RuntimeService) ServiceShutdown() error {
	s.stopPump()
	ctx, cancel := context.WithTimeout(context.Background(), hostv1.CloseBudget)
	defer cancel()
	err := s.host.Close(ctx)
	if s.onShutdownErr != nil {
		s.onShutdownErr(err)
	}
	return err
}

func (s *RuntimeService) stopPump() {
	s.pumpOnce.Do(func() {
		if s.pumpCancel != nil {
			s.pumpCancel()
		}
		if s.pumpDone != nil {
			<-s.pumpDone
		}
	})
}

// pump drains blocking Next and forwards notifications on vivy:event. A
// sticky gap and terminal loss are reported as bridge events.
func (s *RuntimeService) pump(ctx context.Context) {
	defer close(s.pumpDone)
	for {
		batch, err := s.host.Next(ctx, nextBatchLimit)
		if err != nil {
			if ctx.Err() != nil {
				return
			}
			s.emit(eventChannel, wireEvent{Kind: "bridge", Status: "lost"})
			return
		}
		if batch.Gap {
			s.emit(eventChannel, wireEvent{Kind: "bridge", Status: "gap"})
		}
		for _, n := range batch.Events {
			s.emit(eventChannel, wireEvent{Kind: "vivy", Method: n.Method, Params: n.Params})
		}
	}
}

// VivyCall is the single bound RPC forward — verbatim pass-through to the
// host with the requested timeout.
func (s *RuntimeService) VivyCall(ctx context.Context, req CallRequest) CallReply {
	if req.Method == "" {
		return CallReply{Error: &hostv1.Error{Kind: "invalid_input", Code: -32602, Message: "method is required"}}
	}
	if req.TimeoutMs > 0 {
		var cancel context.CancelFunc
		ctx, cancel = context.WithTimeout(ctx, time.Duration(req.TimeoutMs)*time.Millisecond)
		defer cancel()
	}
	res, err := s.host.Call(ctx, req.Method, req.Params)
	if err != nil {
		return CallReply{Error: hostErrorOf(err)}
	}
	return CallReply{OK: true, Result: res}
}

// DispatchRequest is the generic native-command channel (retained desktop
// commands: speech/credentials/voice assets/pet). DispatchTable fills in W4;
// unknown or unimplemented commands answer not_ready — never a silent noop.
type DispatchRequest struct {
	Command string          `json:"command"`
	Payload json.RawMessage `json:"payload,omitempty"`
}

// DesktopDispatch routes retained native commands, gated on the bound main
// window's native identity (W3-5: privileged bindings deny foreign windows).
func (s *RuntimeService) DesktopDispatch(ctx context.Context, req DispatchRequest) CallReply {
	w, ok := ctx.Value(application.WindowKey).(application.Window)
	if !ok || s.cap == nil {
		return CallReply{Error: &hostv1.Error{Kind: "closed", Code: -32081, Message: "no native window identity"}}
	}
	if _, ok := s.cap.tokenFor(w.ID()); !ok {
		return CallReply{Error: &hostv1.Error{Kind: "closed", Code: -32081, Message: "dispatch denied for this window"}}
	}
	if s.dispatch == nil || req.Command == "" {
		return CallReply{Error: &hostv1.Error{Kind: "not_ready", Code: -32080, Message: "native command has no handler yet (lands in W4)"}}
	}
	h, ok := s.dispatch[req.Command]
	if !ok {
		return CallReply{Error: &hostv1.Error{Kind: "not_ready", Code: -32080, Message: "native command has no handler yet (lands in W4): " + req.Command}}
	}
	res, err := h(ctx, req.Payload)
	if err != nil {
		return CallReply{Error: hostErrorOf(err)}
	}
	return CallReply{OK: true, Result: res}
}

// MediaToken issues the W3-3 media capability token, but only when the
// binding context carries the bound main window's native identity. A forged
// or foreign window gets nothing.
func (s *RuntimeService) MediaToken(ctx context.Context) (string, error) {
	w, ok := ctx.Value(application.WindowKey).(application.Window)
	if !ok || s.cap == nil {
		return "", errors.New("no native window identity")
	}
	tok, ok := s.cap.tokenFor(w.ID())
	if !ok {
		return "", errors.New("media capability denied for this window")
	}
	return tok, nil
}

func hostErrorOf(err error) *hostv1.Error {
	var he *hostv1.Error
	if errors.As(err, &he) {
		return he
	}
	return &hostv1.Error{Kind: "internal", Code: -32086, Message: err.Error()}
}
