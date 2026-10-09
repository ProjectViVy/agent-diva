package desktop

import (
	"errors"
	"testing"
)

func TestLifecycleTeardownTruth(t *testing.T) {
	var emitted []string
	l := newLifecycle(func(name string, _ any) { emitted = append(emitted, name) })

	if !l.admissionOpen.Load() {
		t.Fatal("admission should start open")
	}
	l.closeAdmission()
	if l.admissionOpen.Load() {
		t.Fatal("admission still open after OnShutdown")
	}
	if err := l.teardownErr(); err != nil {
		t.Fatalf("unexpected teardown error before record: %v", err)
	}
	boom := errors.New("host close timed out")
	l.recordTeardown(boom)
	if !errors.Is(l.teardownErr(), boom) {
		t.Fatalf("teardown error not surfaced: %v", l.teardownErr())
	}
}

func TestLifecycleSuccessfulRecordDoesNotEraseError(t *testing.T) {
	l := newLifecycle(func(string, any) {})
	boom := errors.New("speech drain incomplete")
	l.recordTeardown(boom)
	l.recordTeardown(nil)
	if !errors.Is(l.teardownErr(), boom) {
		t.Fatalf("successful teardown record erased earlier failure: %v", l.teardownErr())
	}
}

func TestSecondLaunchBeforeWindowReadyQueuesOneReopen(t *testing.T) {
	l := newLifecycle(func(string, any) {})
	shows := 0
	show := func() { shows++ }
	l.requestReopen(show)
	l.requestReopen(show)
	if shows != 0 {
		t.Fatalf("show calls before window ready = %d, want zero", shows)
	}
	l.markReady(show)
	l.markReady(show)
	if shows != 1 {
		t.Fatalf("show calls after readiness = %d, want one", shows)
	}
}

func TestStartupFailureClearsQueuedReopen(t *testing.T) {
	l := newLifecycle(func(string, any) {})
	shows := 0
	show := func() { shows++ }
	l.requestReopen(show)
	l.failStartup()
	l.markReady(show)
	l.requestReopen(show)
	if shows != 0 || l.admissionOpen.Load() {
		t.Fatalf("startup failure retained handoff: show=%d admission=%t", shows, l.admissionOpen.Load())
	}
}

func TestSecondLaunchDuringShutdownDoesNotReopen(t *testing.T) {
	l := newLifecycle(func(string, any) {})
	shows := 0
	show := func() { shows++ }
	l.markReady(show)
	l.closeAdmission()
	l.requestReopen(show)
	if shows != 0 {
		t.Fatalf("show calls after admission closed = %d, want zero", shows)
	}
}

func TestReopenWithoutWindowIsNoop(t *testing.T) {
	d := &Desktop{emit: func(string, any) {}}
	d.reopen() // must not panic with nil window
}

func TestReopenDoesNotShowAfterAdmissionCloses(t *testing.T) {
	l := newLifecycle(func(string, any) {})
	l.closeAdmission()
	showCalls, focusCalls := 0, 0
	w := &fakeWindow{id: 42, name: "main", showCalls: &showCalls, focusCalls: &focusCalls}
	var shown int
	d := &Desktop{
		window:    w,
		lifecycle: l,
		emit: func(name string, _ any) {
			if name == windowStateEvent {
				shown++
			}
		},
	}
	d.reopen()
	if showCalls != 0 || focusCalls != 0 || shown != 0 {
		t.Fatalf("closed lifecycle reopened window: show=%d focus=%d events=%d", showCalls, focusCalls, shown)
	}
}
