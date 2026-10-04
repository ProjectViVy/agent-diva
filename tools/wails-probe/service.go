package main

import (
	"context"
	"fmt"

	"github.com/wailsapp/wails/v3/pkg/application"
)

// ProbeService is a bound Wails service proving the W3-2/W3-3 contract
// surfaces on the pinned beta.27 API.
type ProbeService struct {
	started context.Context
	stopped chan struct{}
}

func (s *ProbeService) ServiceStartup(ctx context.Context, _ application.ServiceOptions) error {
	s.started = ctx
	s.stopped = make(chan struct{})
	return nil
}

func (s *ProbeService) ServiceShutdown() error {
	if s.stopped != nil {
		close(s.stopped)
	}
	return nil
}

// Identify returns the natively injected calling-window identity.
// beta.27 puts the real webview Window into the binding-call context under
// application.WindowKey; the header behind it is overwritten by
// webViewAssetRequest.Header() so a forged x-wails-window-id cannot win.
func (s *ProbeService) Identify(ctx context.Context) (map[string]any, error) {
	w, ok := ctx.Value(application.WindowKey).(application.Window)
	if !ok || w == nil {
		return nil, fmt.Errorf("no calling window in binding context")
	}
	return map[string]any{"id": w.ID(), "name": w.Name()}, nil
}

// CallerIsMain is the W3-3 capability gate shape: bound methods compare the
// injected window identity against the recorded main window.
func (s *ProbeService) CallerIsMain(ctx context.Context, mainID uint) bool {
	w, ok := ctx.Value(application.WindowKey).(application.Window)
	return ok && w != nil && w.ID() == mainID
}
