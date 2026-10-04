// Standalone feasibility instrumentation; intentionally does not import VIVY.
package main

import (
	"context"
	"embed"
	"encoding/json"
	"fmt"
	"io/fs"
	"net/http"
	"os"
	"sync/atomic"
	"time"

	"github.com/egoist/mygo"
)

//go:embed frontend/index.html
var frontend embed.FS
var observed = newProbe()
var primary, foreign *mygo.Window
var finalQuit atomic.Bool
var shutdownError atomic.Value

// Probe exercises the framework boundary, not the product API/DTO contract.
type Probe struct{}

func (Probe) Identity(ctx context.Context) int {
	if w := mygo.CallerWindow(ctx); w != nil {
		return w.ID()
	}
	return 0
}
func (Probe) Capability(ctx context.Context) (string, error) {
	return observed.issue(Probe{}.Identity(ctx))
}
func (Probe) Wait(ctx context.Context) error { return observed.wait(ctx) }
func (Probe) Stream(ctx context.Context, ch *mygo.Channel[string]) error {
	root, err := observed.begin()
	if err != nil {
		return err
	}
	defer observed.end()
	stop := context.AfterFunc(root, ch.Close)
	defer stop()
	// Intentionally fills flow control so page close must unblock Send.
	payload := string(make([]byte, 32<<10))
	for {
		observed.attempted.Add(1)
		if err := ch.Send(payload); err != nil {
			return err
		}
		observed.sent.Add(1)
		if ctx.Err() != nil {
			return ctx.Err()
		}
	}
}

func configure() {
	mygo.App.SetName("DIVA MyGo Probe")
	mygo.Bind(Probe{})
	mygo.App.OnWindowAllClosed(func() {})
	mygo.App.OnBeforeQuit(func(e *mygo.QuitEvent) {
		if finalQuit.Load() {
			return
		}
		e.PreventDefault()
		go func() {
			ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
			defer cancel()
			if err := observed.shutdown(ctx); err != nil {
				shutdownError.Store(err.Error())
			}
			finalQuit.Store(true)
			mygo.App.Quit()
		}()
	})
	mygo.App.WhenReady(func() {
		files, err := fs.Sub(frontend, "frontend")
		if err != nil {
			panic(err)
		}
		serve := mygo.FileServer(files)
		if err := mygo.Protocol.HandleFunc("mygo", func(w http.ResponseWriter, r *http.Request) {
			if r.URL.Path == "/media" {
				observed.media(w, r)
				return
			}
			serve.ServeHTTP(w, r)
		}); err != nil {
			panic(err)
		}
		primary = mygo.NewWindow(mygo.WindowOptions{Title: "DIVA MyGo Probe — primary", Width: 700, Height: 460, URL: "/"})
		observed.start(primary.ID())
		primary.Page().OnDidNavigate(func(string) { observed.revoke() })
		primary.OnClosed(observed.revoke)
		primary.OnClose(func(e *mygo.CloseEvent) {
			if !finalQuit.Load() {
				e.PreventDefault()
				primary.Hide()
			}
		})
		primary.Page().SetPermissionHandler(func(mygo.PermissionRequest) bool { return false })
		foreign = mygo.NewWindow(mygo.WindowOptions{Title: "DIVA MyGo Probe — foreign", Width: 500, Height: 320, URL: "/"})
		foreign.Page().SetPermissionHandler(func(mygo.PermissionRequest) bool { return false })
	})
}
func run() error { configure(); return mygo.App.Run() }
func report() {
	_ = json.NewEncoder(os.Stdout).Encode(map[string]any{"probe": "mygo-v0.2.4", "rootAcquisitions": observed.acquisitions.Load(), "rootCloses": observed.closes.Load(), "activeCalls": observed.active.Load(), "shutdownError": shutdownError.Load(), "nativeReady": mygo.App.IsReady(), "windows": len(mygo.Windows())})
}
func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	report()
}
