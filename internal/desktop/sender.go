package desktop

import (
	"net/http"
	"strconv"
	"strings"
	"sync"
)

const (
	headerWindowID   = "x-wails-window-id"   // natively overwritten by webViewAssetRequest.Header()
	headerWindowName = "x-wails-window-name" // natively overwritten as well
)

// senderGate is the AssetServer-side counterpart of the binding-context gate.
// Every request served to the webview carries the *native* window id the
// platform injected, plus the page Origin/Referer. Anything that did not
// come from our own bundled-asset origin, or claims a window we did not
// grant, is denied here before it reaches assets, /wails/runtime or /media.
type senderGate struct {
	mu           sync.RWMutex
	mainWindowID uint
	allowed      map[uint]bool
	// allowedOrigins: origins that may reach the asset server. The internal
	// scheme appears as http://wails.local on Windows and wails://wails on
	// unix webviews; an empty Origin (top-level navigation of the internal
	// scheme) is allowed because bundled pages never send one cross-scheme.
	allowedOrigins map[string]bool
}

func newSenderGate(mainID uint, extra ...uint) *senderGate {
	g := &senderGate{
		mainWindowID: mainID,
		allowed:      map[uint]bool{mainID: true},
		allowedOrigins: map[string]bool{
			"":                       true,
			"http://wails.local":     true,
			"wails://wails":          true,
			"http://wails.localhost": true,
		},
	}
	for _, id := range extra {
		g.allowed[id] = true
	}
	return g
}

// allow grants an additional native window id (issued only to windows this
// process created).
func (g *senderGate) allow(id uint) {
	g.mu.Lock()
	g.allowed[id] = true
	g.mu.Unlock()
}

func (g *senderGate) originOK(r *http.Request) bool {
	origin := r.Header.Get("Origin")
	if origin == "" {
		origin = r.Header.Get("Referer")
	}
	if origin == "" {
		return true // bundled pages issue same-scheme fetches without Origin
	}
	origin = strings.TrimRight(origin, "/")
	return g.allowedOrigins[origin]
}

func (g *senderGate) windowOK(r *http.Request) bool {
	raw := r.Header.Get(headerWindowID)
	if raw == "" {
		// No injected window id: only possible for non-webview transports.
		return false
	}
	id, err := strconv.ParseUint(raw, 10, 32)
	if err != nil {
		return false
	}
	g.mu.RLock()
	defer g.mu.RUnlock()
	return g.allowed[uint(id)]
}

// Middleware enforces the gate then delegates to next (the Wails default
// asset-server chain).
func (g *senderGate) Middleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !g.originOK(r) {
			http.Error(w, "forbidden origin", http.StatusForbidden)
			return
		}
		if !g.windowOK(r) {
			http.Error(w, "forbidden window", http.StatusForbidden)
			return
		}
		next.ServeHTTP(w, r)
	})
}
