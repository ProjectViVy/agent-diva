package main

import (
	"bytes"
	"context"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestMediaAuthorityAndRawBody(t *testing.T) {
	p := newProbe()
	p.start(7)
	cap, err := p.issue(7)
	if err != nil {
		t.Fatal(err)
	}
	wav := []byte{82, 73, 70, 70, 38, 0, 0, 0, 87, 65, 86, 69, 102, 109, 116, 32, 16, 0, 0, 0, 1, 0, 1, 0, 64, 31, 0, 0, 128, 62, 0, 0, 2, 0, 16, 0, 100, 97, 116, 97, 2, 0, 0, 0, 255, 0}
	tests := []struct {
		name, remote, token string
		want                int
	}{
		{"primary", "mygo:7", cap, 200},
		{"foreign capability replay", "mygo:8", cap, 403},
		{"missing native sender", "127.0.0.1:7", cap, 403},
		{"malformed native sender", "mygo:7:8", cap, 403},
		{"forged capability", "mygo:7", "forged", 403},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := httptest.NewRequest(http.MethodPost, "mygo://localhost/media", bytes.NewReader(wav))
			r.RemoteAddr = tt.remote
			r.Header.Set("X-Diva-Media-Token", tt.token)
			r.Header.Set("X-Window-Id", "7")
			r.Header.Set("Origin", "mygo://localhost")
			w := httptest.NewRecorder()
			p.media(w, r)
			if w.Code != tt.want {
				t.Fatalf("status=%d want=%d", w.Code, tt.want)
			}
			if tt.want == 200 && !bytes.Equal(w.Body.Bytes(), wav) {
				t.Fatal("raw body changed")
			}
		})
	}
	p.revoke()
	r := httptest.NewRequest(http.MethodPost, "mygo://localhost/media", bytes.NewReader(wav))
	r.RemoteAddr = "mygo:7"
	r.Header.Set("X-Diva-Media-Token", cap)
	w := httptest.NewRecorder()
	p.media(w, r)
	if w.Code != 403 {
		t.Fatalf("revoked capability accepted: %d", w.Code)
	}
	if _, err := p.issue(8); err == nil {
		t.Fatal("foreign window issued a capability")
	}
	if err := p.shutdown(context.Background()); err != nil {
		t.Fatal(err)
	}
}

func TestPageCancellationKeepsRootAndShutdownIsOnce(t *testing.T) {
	p := newProbe()
	p.start(7)
	ctx, cancel := context.WithCancel(context.Background())
	done := make(chan error, 1)
	go func() { done <- p.wait(ctx) }()
	cancel()
	if err := <-done; err != context.Canceled {
		t.Fatalf("page canceled: %v", err)
	}
	if p.root.Err() != nil {
		t.Fatal("page canceled application root")
	}
	if err := p.shutdown(context.Background()); err != nil {
		t.Fatal(err)
	}
	if err := p.shutdown(context.Background()); err != nil {
		t.Fatal(err)
	}
	if p.acquisitions.Load() != 1 || p.closes.Load() != 1 {
		t.Fatal("root not acquired/closed once")
	}
	if err := p.wait(context.Background()); err == nil {
		t.Fatal("call admitted after shutdown")
	}
}
