package main

import (
	"context"
	"crypto/rand"
	"errors"
	"io"
	"net/http"
	"strconv"
	"strings"
	"sync"
	"sync/atomic"
)

// Probe state is instrumentation, not a VIVY Host or a product adapter.
// media is registered only as a MyGo protocol handler, never as a TCP handler:
// RemoteAddr is authoritative only when assigned by that native transport.
type probe struct {
	mu           sync.Mutex
	root         context.Context
	cancel       context.CancelFunc
	primary      int
	capability   string
	closing      bool
	active       atomic.Int32
	attempted    atomic.Int32
	sent         atomic.Int32
	acquisitions atomic.Int32
	closes       atomic.Int32
	wg           sync.WaitGroup
	finished     chan struct{}
}

func newProbe() *probe { return &probe{} }
func (p *probe) start(primary int) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.root, p.cancel = context.WithCancel(context.Background())
	p.primary = primary
	p.acquisitions.Add(1)
}
func (p *probe) issue(sender int) (string, error) {
	p.mu.Lock()
	defer p.mu.Unlock()
	if p.root == nil || p.closing || sender != p.primary {
		return "", errors.New("probe: primary window required")
	}
	if p.capability == "" {
		p.capability = rand.Text()
	}
	return p.capability, nil
}
func (p *probe) revoke() { p.mu.Lock(); p.capability = ""; p.mu.Unlock() }
func (p *probe) begin() (context.Context, error) {
	p.mu.Lock()
	defer p.mu.Unlock()
	if p.root == nil || p.closing {
		return nil, errors.New("probe: root unavailable")
	}
	p.wg.Add(1)
	p.active.Add(1)
	return p.root, nil
}
func (p *probe) end() { p.active.Add(-1); p.wg.Done() }
func (p *probe) wait(ctx context.Context) error {
	root, err := p.begin()
	if err != nil {
		return err
	}
	defer p.end()
	select {
	case <-ctx.Done():
		return ctx.Err()
	case <-root.Done():
		return root.Err()
	}
}
func (p *probe) shutdown(ctx context.Context) error {
	p.mu.Lock()
	if !p.closing {
		p.closing = true
		p.capability = ""
		p.finished = make(chan struct{})
		if p.cancel != nil {
			p.cancel()
		}
		go func() { p.wg.Wait(); p.closes.Add(1); close(p.finished) }()
	}
	done := p.finished
	p.mu.Unlock()
	select {
	case <-done:
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}
func nativeSender(remote string) (int, bool) {
	if !strings.HasPrefix(remote, "mygo:") {
		return 0, false
	}
	id, err := strconv.Atoi(strings.TrimPrefix(remote, "mygo:"))
	return id, err == nil && id > 0
}
func (p *probe) media(w http.ResponseWriter, r *http.Request) {
	sender, ok := nativeSender(r.RemoteAddr)
	p.mu.Lock()
	allowed := ok && !p.closing && sender == p.primary && p.capability != "" && r.Header.Get("X-Diva-Media-Token") == p.capability
	p.mu.Unlock()
	if !allowed {
		http.Error(w, "probe: unauthorized", http.StatusForbidden)
		return
	}
	if r.Method != http.MethodPost {
		http.Error(w, "probe: POST required", http.StatusMethodNotAllowed)
		return
	}
	// Bound even this synthetic fixture; no audio/provider or user data is used.
	b, err := io.ReadAll(io.LimitReader(r.Body, (1<<20)+1))
	if err != nil || len(b) > 1<<20 {
		http.Error(w, "probe: invalid body", http.StatusBadRequest)
		return
	}
	w.Header().Set("Content-Type", "audio/wav")
	w.Write(b)
}
