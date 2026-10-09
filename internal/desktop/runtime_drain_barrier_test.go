package desktop

import (
	"context"
	"errors"
	"testing"
	"time"

	hostv1 "agent-vivy/sdk/host/v1"
	"github.com/wailsapp/wails/v3/pkg/application"
)

func TestRuntimeDrainBarrierWaitsAfterRuntimeShutdownReturns(t *testing.T) {
	entered := make(chan struct{})
	release := make(chan struct{})
	h := &fakeHost{nextFn: func(ctx context.Context, _ int) (hostv1.EventBatch, error) {
		close(entered)
		<-ctx.Done()
		<-release
		return hostv1.EventBatch{}, ctx.Err()
	}}
	svc := NewRuntimeService(h, func(string, any) {})
	if err := svc.ServiceStartup(context.Background(), application.ServiceOptions{}); err != nil {
		t.Fatal(err)
	}
	<-entered

	var registered []application.Service
	registerRuntimeServices(func(service application.Service) {
		registered = append(registered, service)
	}, svc)
	if len(registered) != 2 {
		t.Fatalf("registered %d services, want barrier and runtime", len(registered))
	}
	if _, ok := registered[0].Instance().(*runtimeDrainBarrier); !ok {
		t.Fatalf("first service = %T, want drain barrier registered before runtime", registered[0].Instance())
	}
	if got, ok := registered[1].Instance().(*RuntimeService); !ok || got != svc {
		t.Fatalf("second service = %T, want runtime service registered after barrier", registered[1].Instance())
	}

	ctx, cancel := context.WithTimeout(context.Background(), 40*time.Millisecond)
	defer cancel()
	if err := svc.shutdown(ctx); !errors.Is(err, context.DeadlineExceeded) {
		t.Fatalf("bounded runtime shutdown = %v, want deadline", err)
	}

	barrier := registered[0].Instance().(*runtimeDrainBarrier)
	barrierDone := make(chan error, 1)
	go func() { barrierDone <- barrier.ServiceShutdown() }()
	select {
	case err := <-barrierDone:
		t.Fatalf("drain barrier returned before pump released: %v", err)
	case <-time.After(40 * time.Millisecond):
	}

	close(release)
	select {
	case err := <-barrierDone:
		if !errors.Is(err, context.DeadlineExceeded) {
			t.Fatalf("barrier result = %v, want retained runtime deadline", err)
		}
	case <-time.After(time.Second):
		t.Fatal("drain barrier did not return after teardown completed")
	}
	if !h.isClosed() {
		t.Fatal("runtime worker completed without closing the host")
	}
}
