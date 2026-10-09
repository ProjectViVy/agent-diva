package desktop

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"sync"
	"time"

	hostv1 "agent-vivy/sdk/host/v1"
	"github.com/ProjectViVy/agent-diva/internal/speech"
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
	onShutdownErr   func(error)
	onShutdownStart func()
	closeAdmission  func()

	// cap issues the media token only to the bound main window (W3-3/W3-5).
	cap *mediaCapability

	// dispatch maps native command names to handlers; nil until W4 wires
	// the speech/credential/asset table.
	dispatch map[string]func(ctx context.Context, payload json.RawMessage) (json.RawMessage, error)

	pumpMu     sync.Mutex
	pumpCancel context.CancelFunc
	pumpDone   chan struct{}
	pumpOnce   sync.Once

	stateMu         sync.Mutex
	pumpErr         error
	shutdownErr     error
	shutdownStarted bool
	shutdownOnce    sync.Once
	shutdownDone    chan struct{}

	// onShutdown runs before the host close (W4: speech teardown joins
	// in-flight requests), consuming the exact same deadline.
	onShutdown func(context.Context) error
}

// NewRuntimeService wires a host to an event emitter (app.Event.Emit or a
// test sink).
func NewRuntimeService(host hostAPI, emit func(name string, data any)) *RuntimeService {
	return &RuntimeService{host: host, emit: emit, shutdownDone: make(chan struct{})}
}

func (s *RuntimeService) ServiceStartup(_ context.Context, _ application.ServiceOptions) error {
	ctx, cancel := context.WithCancel(context.Background())
	s.pumpMu.Lock()
	if s.pumpDone != nil {
		s.pumpMu.Unlock()
		cancel()
		return fmt.Errorf("runtime event pump already started")
	}
	s.pumpCancel = cancel
	s.pumpDone = make(chan struct{})
	s.pumpMu.Unlock()
	go s.pump(ctx)
	return nil
}

// ServiceShutdown is the single teardown coordinator: stop the producer
// (pump), then close the host under one deadline and report honestly.
func (s *RuntimeService) ServiceShutdown() error {
	ctx, cancel := context.WithTimeout(context.Background(), hostv1.CloseBudget)
	defer cancel()
	return s.shutdown(ctx)
}

// shutdown elects one worker to own teardown. A caller returns at its
// deadline, while the worker retains the host and pump until producer drain
// makes it safe to close them.
func (s *RuntimeService) shutdown(ctx context.Context) error {
	s.shutdownOnce.Do(func() {
		s.stateMu.Lock()
		s.shutdownStarted = true
		s.stateMu.Unlock()
		if s.closeAdmission != nil {
			s.closeAdmission()
		}
		if s.cap != nil {
			s.cap.revoke()
		}
		if s.onShutdownStart != nil {
			s.onShutdownStart()
		}
		go s.runShutdown(ctx)
	})

	select {
	case <-s.shutdownDone:
		return s.shutdownOutcome()
	case <-ctx.Done():
		select {
		case <-s.shutdownDone:
			return s.shutdownOutcome()
		default:
			return ctx.Err()
		}
	}
}

func (s *RuntimeService) runShutdown(ctx context.Context) {
	joined := s.stopPump(ctx)
	s.waitPumpDrain()
	joined = errors.Join(joined, s.currentPumpError())
	if s.onShutdown != nil {
		if err := s.onShutdown(ctx); err != nil {
			joined = errors.Join(joined, fmt.Errorf("speech shutdown: %w", err))
		}
	}
	if s.host != nil {
		if err := s.host.Close(ctx); err != nil {
			joined = errors.Join(joined, fmt.Errorf("VIVY host close: %w", err))
		}
	}
	s.stateMu.Lock()
	s.shutdownErr = joined
	s.stateMu.Unlock()
	if s.onShutdownErr != nil {
		s.onShutdownErr(joined)
	}
	close(s.shutdownDone)
}

func (s *RuntimeService) stopPump(ctx context.Context) error {
	s.pumpMu.Lock()
	cancel, done := s.pumpCancel, s.pumpDone
	s.pumpMu.Unlock()
	s.pumpOnce.Do(func() {
		if cancel != nil {
			cancel()
		}
	})
	if done == nil {
		return nil
	}
	select {
	case <-done:
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}

func (s *RuntimeService) waitPumpDrain() {
	s.pumpMu.Lock()
	done := s.pumpDone
	s.pumpMu.Unlock()
	if done != nil {
		<-done
	}
}

func (s *RuntimeService) recordPumpError(err error) {
	if err == nil {
		return
	}
	s.stateMu.Lock()
	s.pumpErr = errors.Join(s.pumpErr, err)
	s.stateMu.Unlock()
}

func (s *RuntimeService) currentPumpError() error {
	s.stateMu.Lock()
	defer s.stateMu.Unlock()
	return s.pumpErr
}

func (s *RuntimeService) shutdownOutcome() error {
	s.stateMu.Lock()
	defer s.stateMu.Unlock()
	return s.shutdownErr
}

func (s *RuntimeService) awaitShutdown() error {
	s.stateMu.Lock()
	started := s.shutdownStarted
	s.stateMu.Unlock()
	if !started {
		return nil
	}
	<-s.shutdownDone
	return s.shutdownOutcome()
}

// pump drains blocking Next and forwards notifications on vivy:event. A
// sticky gap and terminal loss are reported as bridge events.
func (s *RuntimeService) pump(ctx context.Context) {
	s.pumpMu.Lock()
	done := s.pumpDone
	s.pumpMu.Unlock()
	if done != nil {
		defer close(done)
	}
	for {
		batch, err := s.host.Next(ctx, nextBatchLimit)
		if ctx.Err() != nil {
			return
		}
		if err != nil {
			s.recordPumpError(fmt.Errorf("event pump next: %w", err))
			if emitErr := s.emitPumpEvent(wireEvent{Kind: "bridge", Status: "lost"}); emitErr != nil {
				s.recordPumpError(emitErr)
			}
			return
		}
		if batch.Gap {
			if err := s.emitPumpEvent(wireEvent{Kind: "bridge", Status: "gap"}); err != nil {
				s.recordPumpError(err)
				return
			}
		}
		for _, n := range batch.Events {
			if err := s.emitPumpEvent(wireEvent{Kind: "vivy", Method: n.Method, Params: n.Params}); err != nil {
				s.recordPumpError(err)
				return
			}
		}
	}
}

func (s *RuntimeService) emitPumpEvent(event wireEvent) (err error) {
	defer func() {
		if value := recover(); value != nil {
			if cause, ok := value.(error); ok {
				err = fmt.Errorf("event pump emitter panic: %w", cause)
			} else {
				err = fmt.Errorf("event pump emitter panic: %v", value)
			}
		}
	}()
	if s.emit != nil {
		s.emit(eventChannel, event)
	}
	return nil
}

// VivyCall is the single bound RPC forward — verbatim pass-through to the
// host with the requested timeout.
func (s *RuntimeService) VivyCall(ctx context.Context, req CallRequest) CallReply {
	if _, authErr := s.authorizeMainWindow(ctx); authErr != nil {
		return CallReply{Error: authErr}
	}
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
	if _, authErr := s.authorizeMainWindow(ctx); authErr != nil {
		return CallReply{Error: authErr}
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
	token, authErr := s.authorizeMainWindow(ctx)
	if authErr != nil {
		return "", authErr
	}
	return token, nil
}

// authorizeMainWindow validates the native Wails window identity against the
// one capability bound by the desktop. Every privileged Go binding uses this
// helper before validating payloads or invoking handlers.
func (s *RuntimeService) authorizeMainWindow(ctx context.Context) (string, *hostv1.Error) {
	w, ok := ctx.Value(application.WindowKey).(application.Window)
	if !ok || s.cap == nil {
		return "", &hostv1.Error{Kind: "closed", Code: -32081, Message: "no native window identity"}
	}
	tok, ok := s.cap.tokenFor(w.ID())
	if !ok {
		return "", &hostv1.Error{Kind: "closed", Code: -32081, Message: "native window authorization denied"}
	}
	return tok, nil
}

func hostErrorOf(err error) *hostv1.Error {
	var he *hostv1.Error
	if errors.As(err, &he) {
		return he
	}
	var se *speech.SpeechError
	if errors.As(err, &se) {
		// Speech failures surface kind+message through the bridge and keep
		// the diva.speech/v1 code set in data — never flattened silently.
		kind := "invalid_input"
		switch se.Code {
		case speech.CodeCancelled:
			kind = "cancelled"
		case speech.CodeTimeout:
			kind = "timeout"
		case speech.CodeNotConfigured, speech.CodeCredentialUnavailable,
			speech.CodeNativeUnavailable, speech.CodeDeviceUnavailable:
			kind = "not_ready"
		case speech.CodeBusy, speech.CodeStaleContext, speech.CodeRevisionConflict,
			speech.CodeProviderError, speech.CodeAssetNotFound,
			speech.CodeInvalidAudio, speech.CodeUnsupportedReference,
			speech.CodeInvalidInput:
			kind = "invalid_input"
		}
		data, _ := json.Marshal(se)
		return &hostv1.Error{Kind: kind, Code: -32090, Message: string(se.Code) + ": " + se.Message, Data: data}
	}
	return &hostv1.Error{Kind: "internal", Code: -32086, Message: err.Error()}
}
