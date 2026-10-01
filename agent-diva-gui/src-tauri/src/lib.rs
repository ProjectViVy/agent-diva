//! DIVA Next Tauri thin shell: owns one `vivy_bridge::Host` for the process
//! lifetime, exposes a single passthrough command `vivy_call`, and pumps all
//! VIVY notifications to every window on `vivy:event`. The shell never
//! interprets business method names — DN-0 native commands are a separate
//! allowlist added by their domain stories.

mod lifecycle;

use std::path::PathBuf;
use std::sync::Arc;

pub use lifecycle::{CloseAction, Phase, Shell};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{Emitter, Manager, Runtime, State};
use vivy_bridge::{BridgeError, BridgeEvent, ErrorKind};

/// `vivy_call` request body — raw JSON-RPC frame per ABI v1.
#[derive(Debug, Deserialize)]
pub struct VivyCallRequest {
    method: String,
    #[serde(default)]
    params: Value,
    #[serde(default)]
    timeout_ms: Option<u32>,
}

/// Serializable copy of `BridgeError` for the command boundary.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeErrorDto {
    kind: String,
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl From<BridgeError> for BridgeErrorDto {
    fn from(e: BridgeError) -> Self {
        let kind = match e.kind {
            ErrorKind::InvalidInput => "invalid_input",
            ErrorKind::IncompatibleAbi => "incompatible_abi",
            ErrorKind::Closed => "closed",
            ErrorKind::AlreadyInitialized => "already_initialized",
            ErrorKind::TransportLost => "transport_lost",
            ErrorKind::Timeout => "timeout",
            ErrorKind::EventGap => "event_gap",
            ErrorKind::Internal => "internal",
            ErrorKind::LoadFailed => "load_failed",
        };
        Self { kind: kind.into(), code: e.code, message: e.message, data: e.data }
    }
}

/// Event envelope emitted on `vivy:event`. VIVY payloads travel unchanged;
/// bridge connection/gap status is a separate `bridge` frame.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum WireEvent {
    Vivy { method: String, params: Value },
    Bridge { status: &'static str },
}

/// The single passthrough command. Synchronous FFI runs on a blocking
/// worker so the UI executor never stalls behind a 120 s call.
#[tauri::command]
async fn vivy_call(
    state: State<'_, Arc<Shell>>,
    request: VivyCallRequest,
) -> Result<Value, BridgeErrorDto> {
    let method = request.method.clone();
    let params = request.params.clone();
    let timeout = request.timeout_ms;
    let shell = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || shell.call(&method, params, timeout))
        .await
        .map_err(|e| BridgeErrorDto {
            kind: "internal".into(),
            code: 0,
            message: format!("call worker failed: {e}"),
            data: None,
        })?
        .map_err(BridgeErrorDto::from)
}

/// `vivy_runtime_dir` resolution: `DIVA_VIVY_RUNTIME` (dev/test) or the
/// bundled `vivy-runtime` resource directory.
fn runtime_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, BridgeError> {
    if let Ok(dir) = std::env::var("DIVA_VIVY_RUNTIME") {
        return Ok(PathBuf::from(dir));
    }
    app.path()
        .resource_dir()
        .map(|d| d.join("vivy-runtime"))
        .map_err(|e| BridgeError {
            kind: ErrorKind::LoadFailed,
            code: 0,
            message: format!("resource dir: {e}"),
            data: None,
        })
}

/// `vivy config` path: `DIVA_VIVY_CONFIG` (dev/test) or
/// `<config_dir>/diva/vivy.yaml`.
fn config_path<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, BridgeError> {
    if let Ok(p) = std::env::var("DIVA_VIVY_CONFIG") {
        return Ok(PathBuf::from(p));
    }
    app.path()
        .app_config_dir()
        .map(|d| d.join("vivy.yaml"))
        .map_err(|e| BridgeError {
            kind: ErrorKind::LoadFailed,
            code: 0,
            message: format!("config dir: {e}"),
            data: None,
        })
}

pub fn run() {
    let shell = Arc::new(Shell::new());
    let setup_shell = shell.clone();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .manage(shell.clone() as Arc<Shell>)
        .invoke_handler(tauri::generate_handler![vivy_call])
        .setup(move |app| {
            let handle = app.handle().clone();
            let emit_handle = handle.clone();
            let sink = move |event: BridgeEvent| {
                let wire = match event {
                    BridgeEvent::Vivy(n) => WireEvent::Vivy { method: n.method, params: n.params },
                    BridgeEvent::Gap => WireEvent::Bridge { status: "gap" },
                    BridgeEvent::TransportLost(_) => WireEvent::Bridge { status: "lost" },
                };
                let _ = emit_handle.emit("vivy:event", wire);
            };
            // Fatal startup failure is a process failure: no isolated
            // backend restart inside the same process.
            let dir = runtime_dir(&handle).unwrap_or_else(|e| fatal(e));
            let cfg = config_path(&handle).unwrap_or_else(|e| fatal(e));
            if let Err(e) = setup_shell.start(&dir, cfg, sink) {
                fatal(e);
            }

            // Tray: Quit is the only explicit exit path; window close hides.
            let quit = tauri::menu::MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = tauri::menu::MenuBuilder::new(app).item(&quit).build()?;
            let tray_shell = setup_shell.clone();
            tauri::tray::TrayIconBuilder::new()
                .icon(app.default_window_icon().expect("bundled icon").clone())
                .menu(&menu)
                .on_menu_event(move |app, event| {
                    if event.id().as_ref() == "quit" {
                        tray_shell.request_quit();
                        // Close the window; its CloseRequested handler runs
                        // the ordered shutdown then exits.
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.close();
                        }
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event({
            let shell = shell.clone();
            move |window, event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    match shell.window_close_action() {
                        CloseAction::Hide => {
                            api.prevent_close();
                            let _ = window.hide();
                        }
                        CloseAction::Quit => {
                            api.prevent_close();
                            let app = window.app_handle().clone();
                            let shell = shell.clone();
                            std::thread::spawn(move || {
                                shell.shutdown();
                                app.exit(0);
                            });
                        }
                    }
                }
            }
        });

    builder
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("fatal: tauri build: {e}");
            std::process::exit(1);
        })
        .run(move |_app, event| {
            if let tauri::RunEvent::Exit = event {
                // Belt: any exit path that skipped the Quit branch still
                // tears the host down within its grace bound.
                shell.shutdown();
            }
        });
}

fn fatal(e: BridgeError) -> ! {
    eprintln!("fatal: vivy startup: {e}");
    std::process::exit(1);
}
