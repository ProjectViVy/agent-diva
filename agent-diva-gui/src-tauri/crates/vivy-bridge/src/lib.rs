//! vivy-bridge — Rust-side owner of the sealed VIVY shared library.
//!
//! Implements the DIVA embedded ABI v1 (docs/plans/diva-next/
//! backend-separation-contracts.md §4):
//!   - `VivyInit` / `VivyCall` / `VivyPollEvents` / `VivyShutdown` / `VivyFree`
//!   - every non-null return is UTF-8, NUL-terminated, library-allocated;
//!     the bridge copies it once and calls `VivyFree` exactly once
//!   - no Go pointer crosses FFI: handles are `u64` table indices
//!   - envelope `{ok:true,value}|{ok:false,error:{kind,code,message,data?}}`
//!   - the library is loaded once per process and kept resident until
//!     termination; `VivyShutdown` is the only teardown

use std::ffi::{c_char, c_uint, c_ulonglong, CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use libloading::Library;
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

mod abi;
pub use abi::ABI_VERSION;

/// One server-initiated control-plane message.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Notification {
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// One drain of the VIVY notification queue. `gap` is the library's sticky
/// overflow flag: events were dropped since the previous poll.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PollBatch {
    #[serde(default)]
    pub events: Vec<Notification>,
    #[serde(default)]
    pub gap: bool,
}

/// Status frames the bridge emits alongside untouched VIVY notifications.
#[derive(Debug, Clone, PartialEq)]
pub enum BridgeEvent {
    /// A VIVY notification, payload unchanged.
    Vivy(Notification),
    /// The queue overflowed; events were dropped between polls.
    Gap,
    /// The poll loop failed; the transport is considered lost.
    TransportLost(String),
}

/// ABI error kinds frozen by the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    InvalidInput,
    IncompatibleAbi,
    Closed,
    AlreadyInitialized,
    TransportLost,
    Timeout,
    EventGap,
    Internal,
    /// Raised bridge-side only: the artifact could not be loaded at all.
    #[serde(skip)]
    LoadFailed,
}

/// One bridge failure. `kind` mirrors the envelope `kind` vocabulary.
#[derive(Debug, Error)]
#[error("{kind:?}: {message}")]
pub struct BridgeError {
    pub kind: ErrorKind,
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl BridgeError {
    fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self { kind, code: 0, message: message.into(), data: None }
    }
}

type VivyInitFn = unsafe extern "C" fn(*mut c_char) -> *mut c_char;
type VivyCallFn = unsafe extern "C" fn(c_ulonglong, *mut c_char) -> *mut c_char;
type VivyPollFn = unsafe extern "C" fn(c_ulonglong, c_uint) -> *mut c_char;
type VivyShutdownFn = unsafe extern "C" fn(c_ulonglong) -> *mut c_char;
type VivyFreeFn = unsafe extern "C" fn(*mut c_char);

/// The loaded library plus its resolved symbols. Held `Arc`-shared by every
/// `Host`; the process keeps it resident until termination (no dlclose).
pub struct VivyLibrary {
    _lib: Library,
    /// `VIVY_ABI_VERSION` parsed from the bundled `vivy_abi.h` — the
    /// authoritative ABI identity for this artifact.
    expected_abi: u32,
    init: VivyInitFn,
    call: VivyCallFn,
    poll: VivyPollFn,
    shutdown: VivyShutdownFn,
    free: VivyFreeFn,
}

impl std::fmt::Debug for VivyLibrary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VivyLibrary").field("expected_abi", &self.expected_abi).finish()
    }
}

impl VivyLibrary {
    /// Loads `dir/vivy-shared{.dll,.so,.dylib}` and requires the bundled
    /// `vivy-shared.h` + `vivy_abi.h` beside it so a stale artifact cannot
    /// be picked up silently.
    pub fn load(dir: &Path) -> Result<Arc<Self>, BridgeError> {
        let lib_path = library_path(dir);
        if !lib_path.is_file() {
            return Err(BridgeError::new(
                ErrorKind::LoadFailed,
                format!("shared library not found at {}", lib_path.display()),
            ));
        }
        for header in ["vivy-shared.h", "vivy_abi.h"] {
            let p = dir.join(header);
            if !p.is_file() {
                return Err(BridgeError::new(
                    ErrorKind::LoadFailed,
                    format!("bundled header missing: {}", p.display()),
                ));
            }
        }
        let header_text = std::fs::read_to_string(dir.join("vivy_abi.h")).map_err(|e| {
            BridgeError::new(ErrorKind::LoadFailed, format!("read vivy_abi.h: {e}"))
        })?;
        let expected_abi = parse_header_abi(&header_text).ok_or_else(|| {
            BridgeError::new(ErrorKind::LoadFailed, "vivy_abi.h defines no VIVY_ABI_VERSION")
        })?;
        let lib = unsafe { Library::new(&lib_path) }.map_err(|e| {
            BridgeError::new(
                ErrorKind::LoadFailed,
                format!("dlopen {}: {e}", lib_path.display()),
            )
        })?;
        unsafe {
            Ok(Arc::new(Self {
                expected_abi,
                init: *lib.get(b"VivyInit").map_err(|e| {
                    BridgeError::new(ErrorKind::LoadFailed, format!("symbol VivyInit: {e}"))
                })?,
                call: *lib.get(b"VivyCall").map_err(|e| {
                    BridgeError::new(ErrorKind::LoadFailed, format!("symbol VivyCall: {e}"))
                })?,
                poll: *lib.get(b"VivyPollEvents").map_err(|e| {
                    BridgeError::new(ErrorKind::LoadFailed, format!("symbol VivyPollEvents: {e}"))
                })?,
                shutdown: *lib.get(b"VivyShutdown").map_err(|e| {
                    BridgeError::new(ErrorKind::LoadFailed, format!("symbol VivyShutdown: {e}"))
                })?,
                free: *lib.get(b"VivyFree").map_err(|e| {
                    BridgeError::new(ErrorKind::LoadFailed, format!("symbol VivyFree: {e}"))
                })?,
                _lib: lib,
            }))
        }
    }
}

/// Extracts `VIVY_ABI_VERSION` from a `vivy_abi.h` body.
fn parse_header_abi(text: &str) -> Option<u32> {
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix("#define VIVY_ABI_VERSION") {
            return rest.trim().parse().ok();
        }
    }
    None
}

fn library_path(dir: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    let name = "vivy-shared.dll";
    #[cfg(target_os = "macos")]
    let name = "vivy-shared.dylib";
    #[cfg(all(unix, not(target_os = "macos")))]
    let name = "vivy-shared.so";
    dir.join(name)
}

/// Raw return guard: copies the library-allocated C string, then frees it
/// exactly once even when the bytes are malformed or not UTF-8.
struct OwnedReturn {
    ptr: *mut c_char,
    free: VivyFreeFn,
}

impl OwnedReturn {
    fn new(ptr: *mut c_char, free: VivyFreeFn) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(Self { ptr, free })
        }
    }
}

impl Drop for OwnedReturn {
    fn drop(&mut self) {
        unsafe { (self.free)(self.ptr) }
    }
}

fn c_input(json: &Value) -> Result<CString, BridgeError> {
    let raw = serde_json::to_vec(json)
        .map_err(|e| BridgeError::new(ErrorKind::Internal, format!("encode request: {e}")))?;
    if raw.len() > (4 << 20) {
        return Err(BridgeError::new(
            ErrorKind::InvalidInput,
            "request exceeds the 4 MiB frame bound",
        ));
    }
    CString::new(raw)
        .map_err(|_| BridgeError::new(ErrorKind::InvalidInput, "request contains a NUL byte"))
}

#[derive(Deserialize)]
struct Envelope {
    ok: bool,
    #[serde(default)]
    value: Option<Value>,
    #[serde(default)]
    error: Option<EnvelopeError>,
}

#[derive(Deserialize)]
struct EnvelopeError {
    kind: ErrorKind,
    #[serde(default)]
    code: i64,
    message: String,
    #[serde(default)]
    data: Option<Value>,
}

fn take_envelope(ptr: *mut c_char, free: VivyFreeFn) -> Result<Value, BridgeError> {
    let Some(ret) = OwnedReturn::new(ptr, free) else {
        return Err(BridgeError::new(
            ErrorKind::TransportLost,
            "library returned a null result",
        ));
    };
    let bytes = unsafe { CStr::from_ptr(ret.ptr) }.to_bytes();
    let text = std::str::from_utf8(bytes).map_err(|_| {
        BridgeError::new(ErrorKind::Internal, "library returned non-UTF-8 output")
    })?;
    let env: Envelope = serde_json::from_str(text).map_err(|e| {
        BridgeError::new(ErrorKind::Internal, format!("malformed envelope: {e}"))
    })?;
    if env.ok {
        return Ok(env.value.unwrap_or(Value::Null));
    }
    let e = env
        .error
        .ok_or_else(|| BridgeError::new(ErrorKind::Internal, "error envelope without error"))?;
    Err(BridgeError { kind: e.kind, code: e.code, message: e.message, data: e.data })
}

/// Parameters for `Host::start`.
pub struct InitConfig {
    /// Path to the VIVY config file consumed by `config.Load` library-side.
    pub config_path: PathBuf,
    /// `without_ears` override; DIVA defaults to ears-less.
    pub without_ears: Option<bool>,
}

/// One live host behind a `u64` FFI handle. `shutdown` is the only teardown
/// and is idempotent; calls made during or after shutdown are rejected
/// `closed` without touching the FFI table again.
pub struct Host {
    lib: Arc<VivyLibrary>,
    handle: u64,
    closed: AtomicBool,
    // Serializes shutdown against in-flight calls so no FFI call races the
    // handle's removal.
    gate: Mutex<()>,
}

#[derive(Deserialize)]
struct InitValue {
    abi_version: u32,
    handle: u64,
}

impl std::fmt::Debug for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Host")
            .field("handle", &self.handle)
            .field("closed", &self.is_closed())
            .finish()
    }
}

impl Host {
    /// `VivyInit`: starts the single process-wide host and asserts the
    /// returned ABI version equals the bundled header constant.
    pub fn start(lib: Arc<VivyLibrary>, config: &InitConfig) -> Result<Self, BridgeError> {
        let mut req = serde_json::json!({
            "config_path": config.config_path,
            "abi_version": lib.expected_abi,
        });
        if let Some(ears) = config.without_ears {
            req["without_ears"] = Value::Bool(ears);
        }
        let input = c_input(&req)?;
        let value = take_envelope(unsafe { (lib.init)(input.into_raw()) }, lib.free)?;
        let init: InitValue = serde_json::from_value(value)
            .map_err(|e| BridgeError::new(ErrorKind::Internal, format!("init value: {e}")))?;
        if init.abi_version != lib.expected_abi {
            return Err(BridgeError::new(
                ErrorKind::IncompatibleAbi,
                format!("library abi_version {}, want {}", init.abi_version, lib.expected_abi),
            ));
        }
        Ok(Self { lib, handle: init.handle, closed: AtomicBool::new(false), gate: Mutex::new(()) })
    }

    /// `VivyCall`: `{method, params, timeout_ms?}` passthrough; the library
    /// owns the JSON-RPC dispatch.
    pub fn call(
        &self,
        method: &str,
        params: Value,
        timeout_ms: Option<u32>,
    ) -> Result<Value, BridgeError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(BridgeError::new(ErrorKind::Closed, "host is closed"));
        }
        let mut req = serde_json::json!({ "method": method, "params": params });
        if let Some(t) = timeout_ms {
            req["timeout_ms"] = Value::from(t);
        }
        let input = c_input(&req)?;
        let _gate = self.gate.lock().expect("host gate poisoned");
        if self.closed.load(Ordering::Acquire) {
            return Err(BridgeError::new(ErrorKind::Closed, "host is closed"));
        }
        take_envelope(unsafe { (self.lib.call)(self.handle, input.into_raw()) }, self.lib.free)
    }

    /// `VivyPollEvents`: drains up to `max` queued notifications (library
    /// clamps to 500 when `max` is 0 or above the bound).
    pub fn poll(&self, max: u32) -> Result<PollBatch, BridgeError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(BridgeError::new(ErrorKind::Closed, "host is closed"));
        }
        let _gate = self.gate.lock().expect("host gate poisoned");
        if self.closed.load(Ordering::Acquire) {
            return Err(BridgeError::new(ErrorKind::Closed, "host is closed"));
        }
        let value =
            take_envelope(unsafe { (self.lib.poll)(self.handle, max) }, self.lib.free)?;
        serde_json::from_value(value)
            .map_err(|e| BridgeError::new(ErrorKind::Internal, format!("poll value: {e}")))
    }

    /// `VivyShutdown`: idempotent, bounded by the host's shutdown grace.
    /// In-flight calls complete first; later calls are rejected `closed`.
    pub fn shutdown(&self) -> Result<(), BridgeError> {
        if self.closed.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        let _gate = self.gate.lock().expect("host gate poisoned");
        take_envelope(unsafe { (self.lib.shutdown)(self.handle) }, self.lib.free)?;
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    /// FFI handle, for diagnostics only.
    pub fn raw_handle(&self) -> u64 {
        self.handle
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        let _gate = self.gate.lock().expect("host gate poisoned");
        let ptr = unsafe { (self.lib.shutdown)(self.handle) };
        // Drop frees the envelope even when shutdown reported an error.
        drop(take_envelope(ptr, self.lib.free));
    }
}

/// Poll cadence for the single event-drain worker.
pub const EVENT_PUMP_INTERVAL: std::time::Duration = std::time::Duration::from_millis(25);
/// Max events per `VivyPollEvents` call (ABI bound is 500).
pub const EVENT_PUMP_BATCH: u32 = 500;

/// The one process-wide event drain: polls the library on a dedicated
/// blocking thread and forwards every notification plus bridge status
/// frames to `sink`. The frontend never spawns a second poller.
///
/// The worker exits when `stop` is set or the host closes; a poll failure
/// emits [`BridgeEvent::TransportLost`] once and then keeps polling so a
/// transient error cannot silently kill the stream.
pub fn spawn_event_pump(
    host: Arc<Host>,
    stop: Arc<AtomicBool>,
    mut sink: impl FnMut(BridgeEvent) + Send + 'static,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("vivy-event-pump".into())
        .spawn(move || {
            let mut lost_reported = false;
            while !stop.load(Ordering::Acquire) && !host.is_closed() {
                match host.poll(EVENT_PUMP_BATCH) {
                    Ok(batch) => {
                        lost_reported = false;
                        let idle = !batch.gap && batch.events.is_empty();
                        if batch.gap {
                            sink(BridgeEvent::Gap);
                        }
                        for event in batch.events {
                            sink(BridgeEvent::Vivy(event));
                        }
                        if idle {
                            std::thread::sleep(EVENT_PUMP_INTERVAL);
                        }
                    }
                    Err(err) if err.kind == ErrorKind::Closed => break,
                    Err(err) => {
                        if !lost_reported {
                            sink(BridgeEvent::TransportLost(err.to_string()));
                            lost_reported = true;
                        }
                        std::thread::sleep(EVENT_PUMP_INTERVAL);
                    }
                }
            }
        })
        .expect("spawn vivy-event-pump")
}
