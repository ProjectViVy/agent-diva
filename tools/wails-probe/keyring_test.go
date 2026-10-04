package main

import (
	"errors"
	"testing"

	"github.com/zalando/go-keyring"
)

// W0 Task 5 (Linux leg): the probe must classify backend availability and
// never silently fall back. On headless Linux without a Secret Service
// daemon the set call must ERROR — that error is the recorded "unavailable"
// evidence, not a crash and not a plaintext fallback.
func TestKeyringProbeNeverFallsBack(t *testing.T) {
	res := keyringProbe()
	if res["fallback"] == nil {
		t.Fatal("missing fallback declaration")
	}
	avail, _ := res["available"].(bool)
	if avail {
		// Secret Service present: full set/get/delete cycle must have worked.
		for _, k := range []string{"set", "get", "delete"} {
			if res[k] != "ok" {
				t.Fatalf("available backend but %s=%v", k, res[k])
			}
		}
	} else if res["set"] == "ok" {
		t.Fatal("set succeeded but probe reports unavailable")
	}
}

func TestKeyringNotFoundIsTyped(t *testing.T) {
	_, err := keyring.Get(keyringService, "definitely-absent-account")
	switch {
	case err == nil:
		t.Fatal("absent account read without error")
	case errors.Is(err, keyring.ErrNotFound):
		// typed not-found confirmed
	default:
		t.Skipf("backend unavailable (%v); cannot check typed not-found", err)
	}
}
