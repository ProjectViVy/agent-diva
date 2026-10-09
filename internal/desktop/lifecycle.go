package desktop

import (
	"errors"
	"sync"
	"sync/atomic"

	"github.com/wailsapp/wails/v3/pkg/application"
	"github.com/wailsapp/wails/v3/pkg/events"
)

// windowStateEvent tells the frontend to invalidate per-window state
// (voice context, JS audio) on hide/reload and to resubscribe on reopen.
const windowStateEvent = "vivy:window"

// lifecycle tracks desktop teardown truthfully: admission closes at
// OnShutdown, the service coordinator unwinds producers + host, and any
// incomplete step is reported by PostShutdown.
type lifecycle struct {
	mu            sync.Mutex
	admissionOpen atomic.Bool
	ready         bool
	pendingReopen bool
	startupFailed bool
	teardownDone  bool
	serviceErr    error
	emit          func(name string, data any)
}

func newLifecycle(emit func(name string, data any)) *lifecycle {
	l := &lifecycle{emit: emit}
	l.admissionOpen.Store(true)
	return l
}

// closeAdmission is the Options.OnShutdown hook: from here on no new work
// is admitted; teardown is owned by the service coordinator.
func (l *lifecycle) closeAdmission() {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.admissionOpen.Store(false)
	l.pendingReopen = false
}

// requestReopen coalesces second-instance launches until the primary
// window is ready. Calls that arrive after shutdown or startup failure are
// ignored. The callback runs under the state lock so shutdown cannot race a
// queued show/focus after admission has closed.
func (l *lifecycle) requestReopen(show func()) {
	l.mu.Lock()
	defer l.mu.Unlock()
	if !l.admissionOpen.Load() || l.startupFailed {
		return
	}
	if !l.ready {
		l.pendingReopen = true
		return
	}
	if show != nil {
		show()
	}
}

// markReady releases one queued reopen after the owner has finished
// composition and bound its native window.
func (l *lifecycle) markReady(show func()) {
	l.mu.Lock()
	defer l.mu.Unlock()
	if !l.admissionOpen.Load() || l.startupFailed {
		return
	}
	l.ready = true
	if !l.pendingReopen {
		return
	}
	l.pendingReopen = false
	if show != nil {
		show()
	}
}

// failStartup discards queued handoffs and closes admission for a primary
// that never became ready.
func (l *lifecycle) failStartup() {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.startupFailed = true
	l.ready = false
	l.pendingReopen = false
	l.admissionOpen.Store(false)
}

func (l *lifecycle) beginTeardown() {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.admissionOpen.Store(false)
	l.pendingReopen = false
	l.teardownDone = false
}

// recordTeardown is the Options.PostShutdown hook: last word before exit,
// reports honestly whether the coordinator finished cleanly.
func (l *lifecycle) recordTeardown(err error) {
	l.mu.Lock()
	if err != nil {
		l.serviceErr = errors.Join(l.serviceErr, err)
	}
	l.teardownDone = true
	l.mu.Unlock()
}

func (l *lifecycle) teardownErr() error {
	l.mu.Lock()
	defer l.mu.Unlock()
	return l.serviceErr
}

func (l *lifecycle) teardownState() (bool, error) {
	l.mu.Lock()
	defer l.mu.Unlock()
	return l.teardownDone, l.serviceErr
}

// hideToTray parks the window while the runtime stays alive. Hide
// invalidates window-scoped state: W4 — every in-flight speech request is
// aborted and future admission requires a fresh speech_context_set from the
// reopened window. Shared by the window-close hook and the tray toggle so
// both paths invalidate identically.
func (d *Desktop) hideToTray() {
	if d.lifecycle != nil && !d.lifecycle.admissionOpen.Load() {
		return
	}
	d.window.Hide()
	if d.speech != nil {
		d.speech.InvalidateContext()
	}
	d.emit(windowStateEvent, map[string]any{"state": "hidden"})
}

// installCloseToHide implements W3-4: window close hides and keeps the
// runtime alive. RegisterHook runs synchronously before the internal
// destroy listener (W0-proven), so cancelling WindowClosing prevents the
// webview from being torn down.
func (d *Desktop) installCloseToHide(w application.Window) {
	w.RegisterHook(events.Common.WindowClosing, func(e *application.WindowEvent) {
		e.Cancel()
		d.hideToTray()
	})
}

// reopen brings the hidden window back and tells the frontend to
// resubscribe + refetch projections (no speech replay — W3-4).
func (d *Desktop) reopen() {
	if d.window == nil || d.lifecycle != nil && !d.lifecycle.admissionOpen.Load() {
		return
	}
	d.window.Show()
	d.window.Focus()
	d.emit(windowStateEvent, map[string]any{"state": "shown"})
}
