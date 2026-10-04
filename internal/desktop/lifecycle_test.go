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

func TestReopenWithoutWindowIsNoop(t *testing.T) {
	d := &Desktop{emit: func(string, any) {}}
	d.reopen() // must not panic with nil window
}
