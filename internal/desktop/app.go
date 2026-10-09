// Package desktop is the single Go desktop owner (DN-W3): one Wails app,
// one sealed VIVY host, one window/tray/lifetime coordinator. Nothing here
// imports agent-vivy/internal or registers runtime modules by hand — the
// only VIVY surface is sdk/host/v1.
package desktop

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"fmt"
	"io/fs"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"sync"

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
	assetSlot *handlerSlot
}

type handlerSlot struct {
	mu      sync.RWMutex
	handler http.Handler
}

// runtimeDrainBarrier keeps Wails shutdown inside its service phase until the
// runtime teardown worker has released the host and pump. Wails releases its
// single-instance lease only after every service shutdown callback returns.
type runtimeDrainBarrier struct {
	service *RuntimeService
}

func (*runtimeDrainBarrier) ServiceName() string { return "DIVA runtime drain barrier" }

func (b *runtimeDrainBarrier) ServiceShutdown() error {
	return b.service.awaitShutdown()
}

// registerRuntimeServices relies on Wails' reverse shutdown order: the runtime
// coordinator gets its bounded caller first, then the barrier waits for any
// background drain that outlives that caller before Wails releases its lease.
func registerRuntimeServices(register func(application.Service), service *RuntimeService) {
	register(application.NewService(&runtimeDrainBarrier{service: service}))
	register(application.NewService(service))
}

func (s *handlerSlot) set(handler http.Handler) {
	s.mu.Lock()
	s.handler = handler
	s.mu.Unlock()
}

func (s *handlerSlot) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	s.mu.RLock()
	handler := s.handler
	s.mu.RUnlock()
	if handler == nil {
		http.Error(w, "desktop runtime is not ready", http.StatusServiceUnavailable)
		return
	}
	handler.ServeHTTP(w, r)
}

// Compose elects the Wails process before opening the sealed VIVY host, so a
// secondary launch exits without creating a profile or Journal. A partial
// primary failure closes admission and releases any acquired host owners.
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

	d := &Desktop{logger: logger}
	if err := d.prepare(); err != nil {
		return nil, err
	}
	host, err := hostv1.Open(ctx, hostv1.Options{ConfigPath: cfg.ConfigPath, WithoutEars: cfg.WithoutEars})
	if err != nil {
		d.lifecycle.failStartup()
		return nil, fmt.Errorf("open vivy host: %w", err)
	}
	d.host = host
	if err := d.build(cfg); err != nil {
		d.lifecycle.failStartup()
		cctx, cancel := context.WithTimeout(context.Background(), hostv1.CloseBudget)
		defer cancel()
		return nil, errors.Join(err, d.closePartial(cctx))
	}
	d.lifecycle.markReady(d.reopen)
	return d, nil
}

// prepare creates the singleton application before any runtime/profile
// resources are opened. A second process is terminated by Wails in New.
func (d *Desktop) prepare() error {
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
	d.assetSlot = &handlerSlot{}
	d.app = application.New(application.Options{
		Name:        "DIVA",
		Description: "DIVA desktop (DN-W3 Wails host)",
		Assets: application.AssetOptions{
			Handler:    d.assetSlot,
			Middleware: d.gate.Middleware,
		},
		SingleInstance: &application.SingleInstanceOptions{
			UniqueID: singleInstanceID,
			OnSecondInstanceLaunch: func(data application.SecondInstanceData) {
				d.logger.Printf("second instance rejected; focusing primary window (args=%v)", data.Args)
				d.lifecycle.requestReopen(d.reopen)
			},
		},
		ShouldQuit: func() bool { return true },
		OnShutdown: func() {
			d.lifecycle.closeAdmission()
			d.cap.revoke()
		},
		PostShutdown: func() {
			finished, err := d.lifecycle.teardownState()
			switch {
			case !finished:
				d.logger.Print("teardown pending; process retains ownership until cleanup completes")
			case err != nil:
				d.logger.Printf("teardown incomplete: %v", err)
			default:
				d.logger.Print("teardown complete")
			}
		},
	})
	return nil
}

// build completes runtime composition after Wails has elected the process.
func (d *Desktop) build(cfg Config) error {
	// W4: the Go speech service lives beside vivy.yaml in the config dir.
	speechDir := filepath.Join(filepath.Dir(cfg.ConfigPath), "speech")
	speechSvc, err := speech.OpenService(speechDir, func(dg speech.Diagnostic) {
		d.emit("speech:diagnostic", dg)
	})
	if err != nil {
		return fmt.Errorf("open speech service: %w", err)
	}
	d.speech = speechSvc
	d.service = NewRuntimeService(d.host, d.emit)
	d.service.cap = d.cap
	d.service.closeAdmission = d.lifecycle.closeAdmission
	d.service.onShutdownStart = d.lifecycle.beginTeardown
	d.service.onShutdownErr = d.lifecycle.recordTeardown
	d.service.dispatch = speechDispatch(speechSvc)
	// Speech teardown consumes the coordinator's shared CloseBudget.
	d.service.onShutdown = func(ctx context.Context) error {
		rep := speechSvc.ShutdownContext(ctx)
		if rep.Remaining > 0 {
			return fmt.Errorf("%d request(s) not joined (inflight=%d joined=%d)",
				rep.Remaining, rep.InflightAtStart, rep.Joined)
		}
		d.logger.Printf("speech teardown: inflight=%d joined=%d", rep.InflightAtStart, rep.Joined)
		return nil
	}
	d.assetSlot.set(NewMediaMux(cfg.Frontend, d.cap, d.speech))
	registerRuntimeServices(d.app.RegisterService, d.service)

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
	// Reopen/quit affordances (W5 F2/F3): the close button hides, so the tray
	// is the way back and the way out. AttachWindow + a custom click handler
	// keep left-click toggling through the same hide/reopen semantics as
	// window close; the menu gives explicit Show and Quit entries.
	d.tray.AttachWindow(w)
	d.tray.OnClick(func() {
		if w.IsVisible() {
			d.hideToTray()
		} else {
			d.reopen()
		}
	})
	menu := d.app.NewMenu()
	menu.Add("Show DIVA").OnClick(func(*application.Context) { d.reopen() })
	menu.AddSeparator()
	menu.Add("Quit DIVA").OnClick(func(*application.Context) { d.app.Quit() })
	d.tray.SetMenu(menu)
	return nil
}

func (d *Desktop) closePartial(ctx context.Context) error {
	var errs []error
	if d.speech != nil {
		rep := d.speech.ShutdownContext(ctx)
		if rep.Remaining > 0 {
			errs = append(errs, fmt.Errorf("speech cleanup: %d request(s) not joined (inflight=%d joined=%d)",
				rep.Remaining, rep.InflightAtStart, rep.Joined))
		}
	}
	if d.host != nil {
		if err := d.host.Close(ctx); err != nil {
			errs = append(errs, fmt.Errorf("VIVY host cleanup: %w", err))
		}
	}
	return errors.Join(errs...)
}

// Run blocks until the application exits. On exit it surfaces an incomplete
// teardown as an error — truthfully, per W3-4.
func (d *Desktop) Run() error {
	runErr := d.app.Run()
	var shutdownErr error
	if d.service != nil {
		shutdownErr = d.service.awaitShutdown()
	}
	teardownErr := d.lifecycle.teardownErr()
	if teardownErr == nil {
		teardownErr = shutdownErr
	}
	return errors.Join(runErr, teardownErr)
}

// Host exposes the runtime owner for tests and diagnostics.
func (d *Desktop) Host() *hostv1.Host { return d.host }
