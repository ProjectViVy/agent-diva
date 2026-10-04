package desktop

import (
	"bytes"
	"context"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"runtime"
	"strconv"
	"strings"
	"testing"
)

const testWindowID = 42

func wavBytes(size int) []byte {
	b := []byte("RIFF" + "0000" + "WAVE")
	if len(b) < size {
		b = append(b, bytes.Repeat([]byte{0xAB}, size-len(b))...)
	}
	return b
}

func newReq(method, target string, body []byte, windowID uint, token string) *http.Request {
	var rdr io.Reader
	if body != nil {
		rdr = bytes.NewReader(body)
	}
	req := httptest.NewRequest(method, target, rdr)
	if windowID != 0 {
		req.Header.Set(headerWindowID, strconv.FormatUint(uint64(windowID), 10))
	}
	if token != "" {
		req.Header.Set("X-Diva-Media-Token", token)
	}
	return req
}

// ---- sender gate (W3-3 identity) ----

func TestSenderGateDeniesForeignOrigin(t *testing.T) {
	next := http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) { w.WriteHeader(http.StatusOK) })
	for _, tc := range []struct {
		name   string
		origin string
		want   int
	}{
		{"no origin (bundled page)", "", http.StatusOK},
		{"internal scheme http://wails.local", "http://wails.local", http.StatusOK},
		{"internal scheme wails://wails", "wails://wails", http.StatusOK},
		{"external https origin", "https://evil.example", http.StatusForbidden},
		{"data origin", "data:text/html,x", http.StatusForbidden},
	} {
		req := newReq(http.MethodGet, "/", nil, testWindowID, "")
		if tc.origin != "" {
			req.Header.Set("Origin", tc.origin)
		}
		rec := httptest.NewRecorder()
		newSenderGate(testWindowID).Middleware(next).ServeHTTP(rec, req)
		if rec.Code != tc.want {
			t.Fatalf("%s: got %d want %d", tc.name, rec.Code, tc.want)
		}
	}
}

func TestSenderGateDeniesForeignAndMissingWindows(t *testing.T) {
	next := http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) { w.WriteHeader(http.StatusOK) })
	gate := newSenderGate(testWindowID)

	// foreign window id in the (natively injected) header → deny
	req := newReq(http.MethodGet, "/", nil, 7, "")
	rec := httptest.NewRecorder()
	gate.Middleware(next).ServeHTTP(rec, req)
	if rec.Code != http.StatusForbidden {
		t.Fatalf("foreign window: got %d", rec.Code)
	}

	// missing window header entirely → deny (no webview identity)
	req = newReq(http.MethodGet, "/", nil, 0, "")
	rec = httptest.NewRecorder()
	gate.Middleware(next).ServeHTTP(rec, req)
	if rec.Code != http.StatusForbidden {
		t.Fatalf("missing window: got %d", rec.Code)
	}

	// forged non-numeric window header → deny
	req = newReq(http.MethodGet, "/", nil, 0, "")
	req.Header.Set(headerWindowID, "forged")
	rec = httptest.NewRecorder()
	gate.Middleware(next).ServeHTTP(rec, req)
	if rec.Code != http.StatusForbidden {
		t.Fatalf("forged window header: got %d", rec.Code)
	}
}

// ---- media capability (W3-3 capability) ----

func TestMediaCapabilityTokenWindowAndRevocation(t *testing.T) {
	cap := newMediaCapability(testWindowID, "tok")

	// missing token → deny
	req := newReq(http.MethodPost, "/media/wav", wavBytes(64), testWindowID, "")
	if cap.grant(req) {
		t.Fatal("missing token granted")
	}
	// wrong token → deny
	req = newReq(http.MethodPost, "/media/wav", wavBytes(64), testWindowID, "bad")
	if cap.grant(req) {
		t.Fatal("wrong token granted")
	}
	// right token, wrong window → deny
	req = newReq(http.MethodPost, "/media/wav", wavBytes(64), 99, "tok")
	if cap.grant(req) {
		t.Fatal("wrong window granted")
	}
	// right token + right window → grant
	req = newReq(http.MethodPost, "/media/wav", wavBytes(64), testWindowID, "tok")
	if !cap.grant(req) {
		t.Fatal("main window denied")
	}
	// revoked → deny
	cap.revoke()
	if cap.grant(req) {
		t.Fatal("revoked capability granted")
	}
}

// ---- media route (W3-3 binary route, no TCP listener) ----

func TestMediaRouteWAVRoundTripRawBytes(t *testing.T) {
	cap := newMediaCapability(testWindowID, "tok")
	mux := NewMediaMux(nil, cap)
	body := wavBytes(256)

	req := newReq(http.MethodPost, "/media/wav", body, testWindowID, "tok")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("upload: got %d body %s", rec.Code, rec.Body.String())
	}
	var meta struct {
		ID     string `json:"id"`
		Bytes  int    `json:"bytes"`
		SHA256 string `json:"sha256"`
	}
	decodeJSON(t, rec.Body.Bytes(), &meta)
	if meta.Bytes != len(body) {
		t.Fatalf("bytes recorded %d", meta.Bytes)
	}

	req = newReq(http.MethodGet, "/media/wav/"+meta.ID, nil, testWindowID, "tok")
	rec = httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK || !bytes.Equal(rec.Body.Bytes(), body) {
		t.Fatalf("roundtrip mismatch code=%d", rec.Code)
	}
	if ct := rec.Header().Get("Content-Type"); ct != "audio/wav" {
		t.Fatalf("content type %q", ct)
	}
}

func TestMediaRouteRejectsInvalidWAV(t *testing.T) {
	mux := NewMediaMux(nil, newMediaCapability(testWindowID, "tok"))
	req := newReq(http.MethodPost, "/media/wav", []byte("not-a-wav"), testWindowID, "tok")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("invalid wav: got %d", rec.Code)
	}
}

// W0 Task 3 memory capture: peak live-heap delta around a maximum-size upload.
func TestMediaRoutePeakMemoryAtBound(t *testing.T) {
	cap := newMediaCapability(testWindowID, "tok")
	mux := NewMediaMux(nil, cap)
	body := wavBytes(maxMediaBytes)

	runtime.GC()
	var before runtime.MemStats
	runtime.ReadMemStats(&before)
	req := newReq(http.MethodPost, "/media/wav", body, testWindowID, "tok")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	var after runtime.MemStats
	runtime.ReadMemStats(&after)

	if rec.Code != http.StatusOK {
		t.Fatalf("at-bound upload: got %d", rec.Code)
	}
	delta := int64(after.HeapAlloc) - int64(before.HeapAlloc)
	t.Logf("peak live-heap delta for 10 MiB upload: %d bytes (%.2f MiB)", delta, float64(delta)/(1<<20))
	// The body exists at most ~3x (request body, read buffer, store entry);
	// a JSON/base64 expansion would push this well past 20 MiB.
	if delta > 40<<20 {
		t.Fatalf("live-heap delta %d suggests payload expansion", delta)
	}
}

func TestMediaRouteRejectsOverBound(t *testing.T) {
	mux := NewMediaMux(nil, newMediaCapability(testWindowID, "tok"))
	body := wavBytes(maxMediaBytes + 1)
	req := newReq(http.MethodPost, "/media/wav", body, testWindowID, "tok")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusRequestEntityTooLarge {
		t.Fatalf("over bound: got %d", rec.Code)
	}
}

func TestMediaRouteAtBound(t *testing.T) {
	mux := NewMediaMux(nil, newMediaCapability(testWindowID, "tok"))
	body := wavBytes(maxMediaBytes)
	req := newReq(http.MethodPost, "/media/wav", body, testWindowID, "tok")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusOK {
		t.Fatalf("at bound: got %d body=%.80s", rec.Code, rec.Body.String())
	}
}

func TestMediaRouteDeniedCapability(t *testing.T) {
	mux := NewMediaMux(nil, newMediaCapability(testWindowID, "tok"))
	req := newReq(http.MethodPost, "/media/wav", wavBytes(64), testWindowID, "forged")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, req)
	if rec.Code != http.StatusForbidden {
		t.Fatalf("forged token: got %d", rec.Code)
	}
}

func TestMediaDownloadHonoursCancel(t *testing.T) {
	mux := NewMediaMux(nil, newMediaCapability(testWindowID, "tok"))
	upload := newReq(http.MethodPost, "/media/wav", wavBytes(1<<20), testWindowID, "tok")
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, upload)
	var meta struct {
		ID string `json:"id"`
	}
	decodeJSON(t, rec.Body.Bytes(), &meta)

	ctx, cancel := context.WithCancel(context.Background())
	req := newReq(http.MethodGet, "/media/wav/"+meta.ID, nil, testWindowID, "tok").WithContext(ctx)
	cancel() // cancelled before the handler runs
	rec = httptest.NewRecorder()
	mux.ServeHTTP(rec, req) // must not hang/panic
	if rec.Code != http.StatusOK {
		t.Logf("cancelled request returned %d (acceptable)", rec.Code)
	}
}

func decodeJSON(t *testing.T, b []byte, v any) {
	t.Helper()
	if err := json.Unmarshal(b, v); err != nil {
		t.Fatalf("decode %q: %v", strings.TrimSpace(string(b[:min(len(b), 80)])), err)
	}
}
