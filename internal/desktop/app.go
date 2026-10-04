// Package desktop is the single Go desktop owner (DN-W3): one Wails app,
// one sealed VIVY host, one window/tray/lifetime coordinator. Nothing here
// imports agent-vivy/internal or registers runtime modules by hand — the
// only VIVY surface is sdk/host/v1.
package desktop

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"io/fs"
	"log"
	"os"
	"path/filepath"
	"time"

	hostv1 "agent-vivy/sdk/host/v1"
	"github.com/ProjectViVy/agent-diva/internal/speech"
	"github.com/wailsapp/wails/v3/pkg/application"
)

const singleInstanceID = "foundation.undefine.diva"

// Config selects how the desktop is composed. ConfigPath must resolve to
// an absolute vivy.yaml (see ResolveConfigPath); WithoutEars composes the
// runtime without channel hosts (DIVA has no ears).
type Config struct {
	ConfigPath  string
	WithoutEars bool
	Frontend    fs.FS // bundled UI tree (divagui.Frontend())
}

// Desktop is the composed application — created by Compose, run by Run.
type Desktop struct {
	host      *hostv1.Host
	app       *application.App
	service   *RuntimeService
	gate      *senderGate
	cap       *mediaCapability
	lifecycle *lifecycle
	window    application.Window
	tray      *application.SystemTray
	speech    *speech.Service
	logger    *log.Logger
	emit      func(name string, data any)
}

// Compose opens the sealed VIVY host and builds the Wails app. A partial
// failure closes the host — nothing is published half-ready.
func Compose(ctx context.Context, cfg Config, logger *log.Logger) (*Desktop, error) {
	if logger == nil {
		logger = log.New(os.Stderr, "diva: ", log.LstdFlags)
	}
	if cfg.ConfigPath == "" {
		return nil, fmt.Errorf("config path is required (use ResolveConfigPath)")
	}
	if !filepath.IsAbs(cfg.ConfigPath) {
		return nil, fmt.Errorf("config path must be absolute: %q", cfg.ConfigPath)
	}

	host, err := hostv1.Open(ctx, hostv1.Options{ConfigPath: cfg.ConfigPath, WithoutEars: cfg.WithoutEars})
	if err != nil {
		return nil, fmt.Errorf("open vivy host: %w", err)
	}

	d := &Desktop{host: host, logger: logger}
	if err := d.build(cfg); err != nil {
		cctx, cancel := context.WithTimeout(context.Background(), hostv1.CloseBudget)
		defer cancel()
		_ = d.host.Close(cctx)
		return nil, err
	}
	return d, nil
}

func (d *Desktop) build(cfg Config) error {
	token := make([]byte, 16)
	if _, err := rand.Read(token); err != nil {
		return fmt.Errorf("media capability token: %w", err)
	}
	d.cap = newMediaCapability(0, hex.EncodeToString(token))
	d.gate = newSenderGate(0)

	d.emit = func(name string, data any) {
		if d.app != nil {
			d.app.Event.Emit(name, data)
		}
	}
	d.lifecycle = newLifecycle(d.emit)
	d.service = NewRuntimeService(d.host, d.emit)
	d.service.cap = d.cap
	d.service.onShutdownErr = d.lifecycle.recordTeardown

	// W4: the Go speech service lives beside vivy.yaml in the config dir.
	speechDir := filepath.Join(filepath.Dir(cfg.ConfigPath), "speech")
	speechSvc, err := speech.OpenService(speechDir, func(dg speech.Diagnostic) {
		d.emit("speech:diagnostic", dg)
	})
	if err != nil {
		return fmt.Errorf("open speech service: %w", err)
	}
	d.speech = speechSvc
	d.service.dispatch = speechDispatch(speechSvc)
	// Speech teardown joins in-flight requests before the host close, all
	// under the same CloseBudget; the report is recorded honestly.
	d.service.onShutdown = func() {
		rep := speechSvc.Shutdown(2 * time.Second)
		if rep.Remaining > 0 {
			d.lifecycle.recordTeardown(fmt.Errorf("speech: %d request(s) not joined (inflight=%d joined=%d)",
				rep.Remaining, rep.InflightAtStart, rep.Joined))
		} else {
			d.logger.Printf("speech teardown: inflight=%d joined=%d", rep.InflightAtStart, rep.Joined)
		}
	}

	d.app = application.New(application.Options{
		Name:        "DIVA",
		Description: "DIVA desktop (DN-W3 Wails host)",
		Services:    []application.Service{application.NewService(d.service)},
		Assets: application.AssetOptions{
			Handler:    NewMediaMux(cfg.Frontend, d.cap, d.speech),
			Middleware: d.gate.Middleware,
		},
		SingleInstance: &application.SingleInstanceOptions{
			UniqueID: singleInstanceID,
			OnSecondInstanceLaunch: func(data application.SecondInstanceData) {
				d.logger.Printf("second instance rejected; focusing primary window (args=%v)", data.Args)
				d.reopen()
			},
		},
		ShouldQuit: func() bool { return true },
		OnShutdown: func() {
			d.lifecycle.closeAdmission()
		},
		PostShutdown: func() {
			if err := d.lifecycle.teardownErr(); err != nil {
				d.logger.Printf("teardown incomplete: %v", err)
			} else {
				d.logger.Print("teardown complete")
			}
		},
	})

	w := d.app.Window.NewWithOptions(application.WebviewWindowOptions{
		Name:             "diva-main",
		Title:            "DIVA",
		Width:            1280,
		Height:           800,
		URL:              "/",
		BackgroundColour: application.NewRGB(27, 38, 54),
	})
	d.window = w
	d.gate.allow(w.ID())
	d.cap.bind(w.ID())
	d.installCloseToHide(w)

	d.tray = d.app.SystemTray.New()
	d.tray.SetLabel("DIVA")
	d.tray.SetTooltip("DIVA desktop")
	return nil
}

// Run blocks until the application exits. On exit it surfaces an incomplete
// teardown as an error — truthfully, per W3-4.
func (d *Desktop) Run() error {
	err := d.app.Run()
	if terr := d.lifecycle.teardownErr(); terr != nil && err == nil {
		return fmt.Errorf("teardown: %w", terr)
	}
	return err
}

// Host exposes the runtime owner for tests and diagnostics.
func (d *Desktop) Host() *hostv1.Host { return d.host }
