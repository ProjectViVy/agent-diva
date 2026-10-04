package main

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"sync/atomic"
	"testing"
	"time"

	"github.com/egoist/mygo"
)

func TestMain(m *testing.M) {
	if os.Getenv("MYGO_PROBE_TEST_WATCHDOG") == "1" {
		nativeWatchdog(50 * time.Millisecond)
		select {}
	}

	if os.Getenv("MYGO_PROBE_NATIVE") != "1" {
		os.Exit(m.Run())
	}
	configure()
	watchdog := nativeWatchdog(75 * time.Second)
	defer watchdog.Stop()
	var quitWatchdog atomic.Pointer[time.Timer]
	code := 1
	mygo.App.WhenReady(func() {
		go func() { code = m.Run(); quitWatchdog.Store(nativeWatchdog(10 * time.Second)); mygo.App.Quit() }()
	})
	if err := mygo.App.Run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	if timer := quitWatchdog.Load(); timer != nil {
		timer.Stop()
	}
	watchdog.Stop()
	report()
	if observed.acquisitions.Load() != 1 || observed.closes.Load() != 1 || observed.active.Load() != 0 || shutdownError.Load() != nil {
		fmt.Fprintln(os.Stderr, "probe: quit invariant failed")
		code = 1
	}
	os.Exit(code)
}

// Covers startup and shutdown beyond m.Run's own Go test timeout alarm.
func nativeWatchdog(budget time.Duration) *time.Timer {
	return time.AfterFunc(budget, func() { fmt.Fprintln(os.Stderr, "probe: native event loop timed out"); os.Exit(124) })
}

func TestNativeWatchdogBoundsPostSuiteQuit(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, os.Args[0], "-test.run=^$")
	cmd.Env = append(os.Environ(), "MYGO_PROBE_TEST_WATCHDOG=1")
	b, err := cmd.CombinedOutput()
	exit, ok := err.(*exec.ExitError)
	if !ok || exit.ExitCode() != 124 || !strings.Contains(string(b), "native event loop timed out") {
		t.Fatalf("watchdog failed: %v %s", err, b)
	}
}

func TestGenerationHasNoAcquisitions(t *testing.T) {
	if os.Getenv("MYGO_PROBE_NATIVE") == "1" {
		t.Skip("separate non-native generation scenario")
	}
	output := filepath.Join(t.TempDir(), "client.ts")
	ctx, cancel := context.WithTimeout(context.Background(), 60*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, filepath.Join(runtime.GOROOT(), "bin", "go"), "run", ".")
	cmd.Env = append(os.Environ(), "MYGO_GENERATE="+output)
	b, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("generation: %v\n%s", err, b)
	}
	if !strings.Contains(string(b), `"rootAcquisitions":0`) || !strings.Contains(string(b), `"rootCloses":0`) || !strings.Contains(string(b), `"nativeReady":false`) || !strings.Contains(string(b), `"windows":0`) {
		t.Fatalf("generation acquired resources: %s", b)
	}
	client, err := os.ReadFile(output)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(client), "Probe") || !strings.Contains(string(client), "mygo-runtime") {
		t.Fatal("typed client missing")
	}
}

func waitFor(t *testing.T, label string, fn func() bool) {
	t.Helper()
	deadline := time.Now().Add(15 * time.Second)
	for time.Now().Before(deadline) {
		if fn() {
			return
		}
		time.Sleep(10 * time.Millisecond)
	}
	t.Fatal("timeout: " + label)
}
func eval(t *testing.T, w *mygo.Window, code string) any {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()
	v, err := w.Page().EvalContext(ctx, code)
	if err != nil {
		t.Fatalf("eval %s: %v", code, err)
	}
	return v
}
func loaded(t *testing.T, w *mygo.Window) {
	t.Helper()
	waitFor(t, "native page loaded", func() bool {
		ctx, cancel := context.WithTimeout(context.Background(), time.Second)
		defer cancel()
		v, err := w.Page().EvalContext(ctx, "document.readyState === 'complete' && !!window.mygo && location.protocol !== 'data:'")
		return err == nil && v == true
	})
}
func literal(v any) string { b, _ := json.Marshal(v); return string(b) }
func fetchJS(cap string) string {
	return `fetch('/media',{method:'POST',headers:{'X-Diva-Media-Token':` + literal(cap) + `,'X-Window-Id':` + literal(primary.ID()) + `},body:new Uint8Array([82,73,70,70,38,0,0,0,87,65,86,69,102,109,116,32,16,0,0,0,1,0,1,0,64,31,0,0,128,62,0,0,2,0,16,0,100,97,116,97,2,0,0,0,255,0])}).then(async r=>({status:r.status,bytes:Array.from(new Uint8Array(await r.arrayBuffer()))}))`
}
func status(t *testing.T, w *mygo.Window, cap string, want int) {
	t.Helper()
	v := eval(t, w, fetchJS(cap)).(map[string]any)
	if v["status"] != float64(want) {
		t.Fatalf("media status=%v want=%d", v, want)
	}
	if want == 200 && literal(v["bytes"]) != "[82,73,70,70,38,0,0,0,87,65,86,69,102,109,116,32,16,0,0,0,1,0,1,0,64,31,0,0,128,62,0,0,2,0,16,0,100,97,116,97,2,0,0,0,255,0]" {
		t.Fatalf("raw bytes changed: %v", v)
	}
}
func TestNativeBoundariesAndLifecycle(t *testing.T) {
	if os.Getenv("MYGO_PROBE_NATIVE") != "1" {
		t.Skip("requires a real native desktop session")
	}
	loaded(t, primary)
	loaded(t, foreign)
	t.Run("native caller survives forged JS window id", func(t *testing.T) {
		v := eval(t, foreign, `mygo.windowId=`+literal(primary.ID())+`;return await mygo.call('Probe.Identity')`)
		if v != float64(foreign.ID()) {
			t.Fatalf("caller spoofed: %v", v)
		}
		v = eval(t, foreign, `try{await mygo.call('Probe.Capability');return false}catch(e){return true}`)
		if v != true {
			t.Fatal("foreign capability issued")
		}
	})
	cap := eval(t, primary, `mygo.call('Probe.Capability')`).(string)
	t.Run("native raw WAV and foreign replay", func(t *testing.T) {
		status(t, primary, cap, 200)
		status(t, foreign, cap, 403)
		status(t, primary, "forged", 403)
	})
	t.Run("revoked capability", func(t *testing.T) { observed.revoke(); status(t, primary, cap, 403) })
	cap = eval(t, primary, `mygo.call('Probe.Capability')`).(string)
	t.Run("page reload cancels call and revokes capability but keeps root", func(t *testing.T) {
		eval(t, primary, `window.pending=mygo.call('Probe.Wait').catch(()=>{});return true`)
		waitFor(t, "call started", func() bool { return observed.active.Load() == 1 })
		primary.Page().Reload()
		waitFor(t, "page call canceled", func() bool { return observed.active.Load() == 0 })
		loaded(t, primary)
		status(t, primary, cap, 403)
		if observed.root.Err() != nil || observed.acquisitions.Load() != 1 {
			t.Fatal("reload changed root lifetime")
		}
	})
	t.Run("page channel close unblocks sender and keeps root", func(t *testing.T) {
		eval(t, primary, `window.ch=mygo.channel();window.streaming=mygo.call('Probe.Stream',ch).catch(()=>{});return true`)
		waitFor(t, "stream started", func() bool { return observed.active.Load() == 1 })
		// Do not consume messages. A stable in-progress Send beyond the
		// flow-control budget proves the sender actually blocked before close.
		waitFor(t, "sender reaches flow control", func() bool { return observed.sent.Load() >= 5 && observed.attempted.Load() == observed.sent.Load()+1 })
		before := observed.sent.Load()
		time.Sleep(200 * time.Millisecond)
		if observed.sent.Load() != before || observed.attempted.Load() != before+1 {
			t.Fatal("unread channel did not block Send")
		}

		eval(t, primary, `ch.close();return true`)
		waitFor(t, "stream canceled", func() bool { return observed.active.Load() == 0 })
		if observed.root.Err() != nil {
			t.Fatal("channel close canceled root")
		}
	})
	t.Run("close hides and reopen keeps owner", func(t *testing.T) {
		primary.Close()
		if primary.IsDestroyed() || primary.IsVisible() {
			t.Fatal("close failed to hide")
		}
		primary.Show()
		if !primary.IsVisible() || observed.acquisitions.Load() != 1 {
			t.Fatal("reopen acquired a root")
		}
	})
	t.Run("untrusted external navigation denies bound call and old capability", func(t *testing.T) {
		cap = eval(t, primary, `mygo.call('Probe.Capability')`).(string)
		if err := primary.Page().LoadURL("data:text/html,<title>foreign-origin</title><p>untrusted</p>"); err != nil {
			t.Fatal(err)
		}
		waitFor(t, "external navigation committed", func() bool {
			observed.mu.Lock()
			defer observed.mu.Unlock()
			return observed.capability == ""
		})
		v := eval(t, primary, `try{await mygo.call('Probe.Capability');return 'accepted'}catch(e){return String(e)}`)
		if !strings.Contains(fmt.Sprint(v), "not allowed to call Go methods") {
			t.Fatalf("untrusted call outcome: %v", v)
		}
		if err := primary.Page().LoadURL("/"); err != nil {
			t.Fatal(err)
		}
		loaded(t, primary)
		status(t, primary, cap, 403)
	})
	// Start an outstanding call. TestMain initiates explicit Quit and then checks
	// the actual post-loop cancellation/one-close result (not a simulated hook).
	eval(t, primary, `window.pending=mygo.call('Probe.Wait').catch(()=>{});return true`)
	waitFor(t, "outstanding call before explicit quit", func() bool { return observed.active.Load() == 1 })
}
