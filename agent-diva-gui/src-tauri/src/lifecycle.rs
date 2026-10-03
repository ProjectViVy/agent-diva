//! Native lifecycle ownership for the embedded VIVY host.
//!
//! One process owns the library independent of its windows: the host and the
//! event pump start in `setup`, window close only hides to the tray, and an
//! explicit quit runs the ordered DN-L shutdown (pump stop → drain join →
//! `VivyShutdown`). `VivyCall` is rejected `closed` the moment teardown
//! begins; a fatal startup failure is a process failure, never a restart in
//! the same process.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use serde_json::Value;
use vivy_bridge::{
    spawn_event_pump, BridgeError, BridgeEvent, ErrorKind, Host, InitConfig, VivyLibrary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Host not yet started; calls are rejected `closed`.
    Starting,
    /// Host live; calls and polls flow.
    Ready,
    /// Ordered shutdown in progress; late calls rejected.
    Closing,
    /// Teardown complete.
    Dead,
}

/// What a window close must do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    /// Hide the window (tray-resident); the host and pump keep running.
    Hide,
    /// The user asked to quit: run ordered shutdown, then exit.
    Quit,
}

pub struct Shell {
    phase: Mutex<Phase>,
    quitting: AtomicBool,
    host: Mutex<Option<Arc<Host>>>,
    pump_stop: Arc<AtomicBool>,
    pump: Mutex<Option<JoinHandle<()>>>,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            phase: Mutex::new(Phase::Starting),
            quitting: AtomicBool::new(false),
            host: Mutex::new(None),
            pump_stop: Arc::new(AtomicBool::new(false)),
            pump: Mutex::new(None),
        }
    }

    pub fn phase(&self) -> Phase {
        *self.phase.lock().expect("phase poisoned")
    }

    /// Starts the host and the single event pump. `sink` receives every
    /// forwarded event; the shell maps it onto `vivy:event` per window.
    /// Startup failure is returned to the caller as fatal.
    pub fn start(
        &self,
        runtime_dir: &Path,
        config_path: PathBuf,
        sink: impl FnMut(BridgeEvent) + Send + 'static,
    ) -> Result<(), BridgeError> {
        let lib = VivyLibrary::load(runtime_dir)?;
        let host = Arc::new(Host::start(
            lib,
            &InitConfig { config_path, without_ears: Some(true) },
        )?);
        let pump = spawn_event_pump(host.clone(), self.pump_stop.clone(), sink);
        *self.host.lock().expect("host poisoned") = Some(host);
        *self.pump.lock().expect("pump poisoned") = Some(pump);
        *self.phase.lock().expect("phase poisoned") = Phase::Ready;
        Ok(())
    }

    /// Passthrough `vivy_call`. Rejects every call unless the host is Ready.
    pub fn call(
        &self,
        method: &str,
        params: Value,
        timeout_ms: Option<u32>,
    ) -> Result<Value, BridgeError> {
        let guard = self.host.lock().expect("host poisoned");
        match *self.phase.lock().expect("phase poisoned") {
            Phase::Ready => {}
            Phase::Closing | Phase::Dead => {
                return Err(BridgeError {
                    kind: ErrorKind::Closed,
                    code: 0,
                    message: "host is shutting down".into(),
                    data: None,
                });
            }
            Phase::Starting => {
                return Err(BridgeError {
                    kind: ErrorKind::Closed,
                    code: 0,
                    message: "host is not ready".into(),
                    data: None,
                });
            }
        }
        guard
            .as_ref()
            .expect("ready phase without host")
            .call(method, params, timeout_ms)
    }

    /// Ordered teardown: stop the pump, join it, then `VivyShutdown`.
    /// Idempotent and bounded by the library's shutdown grace.
    pub fn shutdown(&self) {
        let mut phase = self.phase.lock().expect("phase poisoned");
        match *phase {
            Phase::Closing | Phase::Dead => return,
            _ => *phase = Phase::Closing,
        }
        drop(phase);
        self.pump_stop.store(true, Ordering::Release);
        if let Some(pump) = self.pump.lock().expect("pump poisoned").take() {
            let _ = pump.join();
        }
        if let Some(host) = self.host.lock().expect("host poisoned").take() {
            let _ = host.shutdown();
        }
        *self.phase.lock().expect("phase poisoned") = Phase::Dead;
    }

    /// Decide what a window close does. Quit only when the user asked for
    /// it explicitly; otherwise the window hides and the host keeps running.
    pub fn window_close_action(&self) -> CloseAction {
        if self.quitting.load(Ordering::Acquire) {
            CloseAction::Quit
        } else {
            CloseAction::Hide
        }
    }

    /// Begin explicit quit (tray menu / menu Quit): flips the close
    /// semantics so the next window close exits instead of hiding.
    pub fn request_quit(&self) {
        self.quitting.store(true, Ordering::Release);
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}
