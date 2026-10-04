package main

import (
	"errors"
	"fmt"
	"runtime"

	"github.com/zalando/go-keyring"
)

// W0 Task 5: zalando/go-keyring v0.2.8 is the pinned OS keyring dependency.
// Windows → Credential Manager (wincred), macOS → Keychain, Linux → Secret
// Service over D-Bus. The library returns errors for unavailable/locked
// stores; the probe MUST surface them — no plaintext fallback exists here or
// may ever be added in cmd/diva.
const (
	keyringService = "com.projectvivy.wailsprobe"
	keyringAccount = "probe-credential"
)

// keyringProbe exercises set/read/delete and reports backend availability.
// Every failure is returned as data (never swallowed) so the run log records
// the exact platform state.
func keyringProbe() map[string]any {
	out := map[string]any{
		"dependency": "github.com/zalando/go-keyring v0.2.8",
		"platform":   runtime.GOOS,
		"backend":    keyringBackend(runtime.GOOS),
		"fallback":   "none — backend failure is an error, never plaintext",
	}
	const secret = "probe-secret-v1"

	if err := keyring.Set(keyringService, keyringAccount, secret); err != nil {
		out["set"] = "error: " + err.Error()
		out["available"] = false
		return out
	}
	out["set"] = "ok"

	got, err := keyring.Get(keyringService, keyringAccount)
	switch {
	case err != nil:
		out["get"] = "error: " + err.Error()
	case got != secret:
		out["get"] = "mismatch"
	default:
		out["get"] = "ok"
	}

	if err := keyring.Delete(keyringService, keyringAccount); err != nil {
		out["delete"] = "error: " + err.Error()
	} else {
		out["delete"] = "ok"
	}
	if _, err := keyring.Get(keyringService, keyringAccount); errors.Is(err, keyring.ErrNotFound) {
		out["post_delete"] = "not found (expected)"
	} else if err != nil {
		out["post_delete"] = "error: " + err.Error()
	} else {
		out["post_delete"] = "still readable (unexpected)"
	}
	out["available"] = true
	return out
}

func keyringBackend(goos string) string {
	switch goos {
	case "windows":
		return "Windows Credential Manager (wincred)"
	case "darwin":
		return "macOS Keychain (Security.framework)"
	case "linux":
		return "Secret Service over D-Bus (freedesktop)"
	default:
		return "unsupported"
	}
}

// runKeyringProbe is invoked by `wails-probe --keyring`; it prints the probe
// result lines so the log captures them verbatim.
func runKeyringProbe() {
	for k, v := range keyringProbe() {
		fmt.Printf("keyring %-12s %v\n", k+":", v)
	}
}
