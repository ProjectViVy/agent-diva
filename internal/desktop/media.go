package desktop

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"net/http"
	"strconv"
	"sync"
	"testing/fstest"
)

// maxMediaBytes is the W3-3 bound: 10 MiB per media request.
const maxMediaBytes = 10 << 20

// mediaCapability is issued to exactly one native window id; the token is the
// capability the W3-3 media route checks on every call.
type mediaCapability struct {
	mu       sync.Mutex
	windowID uint
	token    string
	revoked  bool
}

func newMediaCapability(windowID uint, token string) *mediaCapability {
	return &mediaCapability{windowID: windowID, token: token}
}

// bind issues the capability to the given native window id — called exactly
// once when the primary window is created.
func (c *mediaCapability) bind(windowID uint) {
	c.mu.Lock()
	c.windowID = windowID
	c.mu.Unlock()
}

// tokenFor returns the capability token only for the bound window — used by
// the privileged MediaToken binding.
func (c *mediaCapability) tokenFor(windowID uint) (string, bool) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.revoked || windowID != c.windowID {
		return "", false
	}
	return c.token, true
}

func (c *mediaCapability) revoke() {
	c.mu.Lock()
	c.revoked = true
	c.mu.Unlock()
}

func (c *mediaCapability) grant(r *http.Request) bool {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.revoked || r.Header.Get("X-Diva-Media-Token") != c.token {
		return false
	}
	id, err := strconv.ParseUint(r.Header.Get(headerWindowID), 10, 32)
	return err == nil && uint(id) == c.windowID
}

// mediaStore keeps uploaded WAV payloads in memory for the probe.
type mediaStore struct {
	mu       sync.Mutex
	payloads map[string][]byte
}

// isWAV checks the RIFF/WAVE magic pair without parsing.
func isWAV(b []byte) bool {
	return len(b) >= 12 &&
		string(b[0:4]) == "RIFF" &&
		string(b[8:12]) == "WAVE"
}

// NewMediaMux returns the asset-server handler: bundled frontend under / and
// the bounded binary media route under /media/wav — all inside the internal
// wails scheme, no TCP listener anywhere.
func NewMediaMux(assets fs.FS, cap *mediaCapability) http.Handler {
	store := &mediaStore{payloads: map[string][]byte{}}
	mux := http.NewServeMux()

	mux.HandleFunc("POST /media/wav", func(w http.ResponseWriter, r *http.Request) {
		if !cap.grant(r) {
			http.Error(w, "media capability denied", http.StatusForbidden)
			return
		}
		r.Body = http.MaxBytesReader(w, r.Body, maxMediaBytes)
		body, err := io.ReadAll(r.Body)
		if err != nil {
			var maxErr *http.MaxBytesError
			if errors.As(err, &maxErr) {
				http.Error(w, "payload too large", http.StatusRequestEntityTooLarge)
				return
			}
			http.Error(w, err.Error(), http.StatusBadRequest)
			return
		}
		if !isWAV(body) {
			http.Error(w, "not a WAV payload", http.StatusBadRequest)
			return
		}
		sum := sha256.Sum256(body)
		id := hex.EncodeToString(sum[:8])
		store.mu.Lock()
		store.payloads[id] = body
		store.mu.Unlock()
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{
			"id": id, "bytes": len(body), "sha256": hex.EncodeToString(sum[:]),
		})
	})

	mux.HandleFunc("GET /media/wav/{id}", func(w http.ResponseWriter, r *http.Request) {
		if !cap.grant(r) {
			http.Error(w, "media capability denied", http.StatusForbidden)
			return
		}
		store.mu.Lock()
		body, ok := store.payloads[r.PathValue("id")]
		store.mu.Unlock()
		if !ok {
			http.NotFound(w, r)
			return
		}
		w.Header().Set("Content-Type", "audio/wav")
		w.Header().Set("Content-Length", strconv.Itoa(len(body)))
		// Response writes honour r.Context(): a native webview cancellation
		// aborts the write (webViewAssetRequest.Context() preserves it).
		_, _ = w.Write(body)
	})

	// Everything else is the bundled frontend (fs may be nil in tests).
	mux.Handle("/", http.FileServerFS(orEmptyFS(assets)))
	return mux
}

func orEmptyFS(f fs.FS) fs.FS {
	if f != nil {
		return f
	}
	return fstest.MapFS{}
}

// mediaHandlerInfo describes the route for the startup log.
func mediaHandlerInfo() string {
	return fmt.Sprintf("POST /media/wav + GET /media/wav/{id}; bound %d bytes; token+window capability; raw bytes, no JSON/base64", maxMediaBytes)
}
