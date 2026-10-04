// wails-probe proves the pinned Wails v3.0.0-beta.27 candidate against the
// W3-2/W3-3 native contract surfaces before DIVA adopts it for the Go host.
// It is deliberately separate from cmd/diva (which does not exist yet).
//
// Headless default: prints the recorded API report and exits 0.
// GUI:        `go run ./tools/wails-probe --gui` opens the probe window
//
//	(needs a desktop; the Windows x64 run is the W0 exit gate).
package main

import (
	"crypto/rand"
	"embed"
	"encoding/hex"
	"flag"
	"fmt"
	"os"
	"runtime"

	"github.com/wailsapp/wails/v3/pkg/application"
	"github.com/wailsapp/wails/v3/pkg/events"
)

//go:embed frontend
var frontendFS embed.FS

func main() {
	gui := flag.Bool("gui", false, "open the probe window (needs a desktop)")
	kr := flag.Bool("keyring", false, "run the OS keyring probe")
	flag.Parse()
	switch {
	case *kr:
		runKeyringProbe()
	case *gui:
		runApp()
	default:
		report()
	}
}

func report() {
	fmt.Println("wails-probe — pinned API report")
	fmt.Println("module:  github.com/wailsapp/wails/v3 v3.0.0-beta.27")
	fmt.Println("runtime: @wailsio/runtime npm 3.0.0-beta.27 (from tagged source package.json)")
	fmt.Printf("platform: %s/%s %s\n", runtime.GOOS, runtime.GOARCH, runtime.Version())
	fmt.Println("app:     application.New(application.Options{Services,Assets,OnShutdown,PostShutdown,ShouldQuit,RawMessageHandler,SingleInstance})")
	fmt.Println("window:  app.Window.NewWithOptions(application.WebviewWindowOptions) -> *WebviewWindow (application.Window)")
	fmt.Println("tray:    app.SystemTray.New() -> *SystemTray{SetIcon,SetMenu,SetLabel,SetTooltip,Show,Hide,Destroy}")
	fmt.Println("events:  app.Event.Emit(name,...)/On(name,cb)/Once/Off; CustomEvent cancellable; window.EmitEvent")
	fmt.Println("sender:  ctx.Value(application.WindowKey).(application.Window).ID() — x-wails-window-id natively overwritten")
	fmt.Println("close2hide: window.RegisterHook(events.Common.WindowClosing, cancel+Hide) — hooks run before listeners")
	fmt.Println("media:   ", mediaHandlerInfo())
	fmt.Println("lifetime: Options.OnShutdown -> ServiceShutdown (reverse registration) -> Options.PostShutdown")
}

func runApp() {
	token := make([]byte, 16)
	_, _ = rand.Read(token)
	cap := newMediaCapability(0, hex.EncodeToString(token))
	gate := newSenderGate(0)

	probe := &ProbeService{}
	var mainWindow application.Window
	app := application.New(application.Options{
		Name:        "wails-probe",
		Description: "DN-W3 W0 native feasibility probe",
		Services:    []application.Service{application.NewService(probe)},
		Assets: application.AssetOptions{
			Handler:    NewMediaMux(frontendFS, cap),
			Middleware: gate.Middleware,
		},
		RawMessageHandler: func(window application.Window, message string, oi *application.OriginInfo) {
			fmt.Printf("rawmsg window=%d origin=%q top=%q mainframe=%v\n", window.ID(), oi.Origin, oi.TopOrigin, oi.IsMainFrame)
		},
		SingleInstance: &application.SingleInstanceOptions{
			UniqueID: "com.projectvivy.wailsprobe",
			OnSecondInstanceLaunch: func(data application.SecondInstanceData) {
				fmt.Printf("second instance denied: %v\n", data.Args)
				if mainWindow != nil {
					mainWindow.Show()
					mainWindow.Focus()
				}
			},
		},
		ShouldQuit:   func() bool { return true },
		OnShutdown:   func() { fmt.Println("OnShutdown: in-flight calls may still be running") },
		PostShutdown: func() { fmt.Println("PostShutdown: process about to exit") },
	})

	w := app.Window.NewWithOptions(application.WebviewWindowOptions{
		Name:             "diva-main",
		Title:            "Wails Probe",
		Width:            960,
		Height:           640,
		URL:              "/",
		BackgroundColour: application.NewRGB(27, 38, 54),
	})
	mainWindow = w
	gate.allowed[w.ID()] = true
	cap.windowID = w.ID()

	// Close-to-hide: hooks run before the internal destroy listener; a
	// cancelled WindowClosing never reaches it, so the webview stays alive.
	w.RegisterHook(events.Common.WindowClosing, func(e *application.WindowEvent) {
		e.Cancel()
		w.Hide()
	})

	tray := app.SystemTray.New()
	tray.SetLabel("probe")
	tray.SetTooltip("wails-probe")

	app.Event.On("probe:tick", func(e *application.CustomEvent) {
		fmt.Println("tick:", e.Data)
	})

	fmt.Printf("main window id=%d name=%q; capability issued to it\n", w.ID(), w.Name())
	if err := app.Run(); err != nil {
		fmt.Fprintln(os.Stderr, "run:", err)
		os.Exit(1)
	}
}
