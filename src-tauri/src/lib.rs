mod bridge;
mod diagnostics;
mod process;
mod storage;
use bridge::Bridge;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{Emitter, Manager};

#[tauri::command]
async fn core_connect(state: tauri::State<'_, Arc<Bridge>>) -> Result<Value, String> {
    let bridge = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || bridge.start())
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn core_request(
    state: tauri::State<'_, Arc<Bridge>>,
    request: Value,
) -> Result<Value, String> {
    let bridge = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || bridge.request(request))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn core_submit(
    state: tauri::State<'_, Arc<Bridge>>,
    record: storage::SendRecord,
) -> Result<Value, String> {
    let bridge = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || bridge.submit(record))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn core_status(state: tauri::State<'_, Arc<Bridge>>) -> Value {
    state.status()
}
#[tauri::command]
fn core_diagnostics(state: tauri::State<'_, Arc<Bridge>>) -> Result<String, String> {
    state.diagnostics()
}
#[tauri::command]
fn onboarding_read(
    state: tauri::State<'_, Mutex<storage::OnboardingStore>>,
) -> Result<Value, String> {
    let store = state.lock().map_err(|_| "onboarding state lock poisoned")?;
    Ok(json!({ "completed": store.value.completed, "forced": store.value.forced }))
}
#[tauri::command]
fn onboarding_complete(
    state: tauri::State<'_, Mutex<storage::OnboardingStore>>,
) -> Result<Value, String> {
    let mut store = state.lock().map_err(|_| "onboarding state lock poisoned")?;
    store.set_completed(true).map_err(|e| e.to_string())?;
    Ok(json!({ "completed": true, "forced": false }))
}
#[tauri::command]
fn onboarding_reset(
    state: tauri::State<'_, Mutex<storage::OnboardingStore>>,
) -> Result<Value, String> {
    let mut store = state.lock().map_err(|_| "onboarding state lock poisoned")?;
    store.reset().map_err(|e| e.to_string())?;
    Ok(json!({ "completed": false, "forced": true }))
}
#[tauri::command]
fn open_persona_directory(state: tauri::State<'_, Arc<Bridge>>) -> Result<String, String> {
    let directory = state.persona_directory()?;
    open_directory(&directory)?;
    Ok(directory.to_string_lossy().into_owned())
}
fn open_directory(directory: &std::path::Path) -> Result<(), String> {
    #[cfg(windows)]
    let mut command = std::process::Command::new("explorer.exe");
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = std::process::Command::new("xdg-open");
    command
        .arg(directory)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("open persona directory: {e}"))
}
fn laputa_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    // Must match vivy-laputa's root_from: LAPUTA_HOME, else the user home.
    if let Some(path) = std::env::var_os("LAPUTA_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    app.path()
        .home_dir()
        .map(|home| home.join(".laputa"))
        .map_err(|e| format!("resolve home directory: {e}"))
}

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show(app);
        }))
        .invoke_handler(tauri::generate_handler![
            core_connect,
            core_request,
            core_submit,
            core_status,
            core_diagnostics,
            onboarding_read,
            onboarding_complete,
            onboarding_reset,
            open_persona_directory
        ])
        .setup(|app| {
            let root = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(root.join("webview2"))?;
            let laputa = laputa_root(app.handle()).map_err(std::io::Error::other)?;
            let onboarding = storage::OnboardingStore::open(&root.join("desktop-state"))
                .map_err(|e| std::io::Error::other(format!("onboarding state: {e}")))?;
            app.manage(Mutex::new(onboarding));
            let events = app.handle().clone();
            let bridge = Bridge::new(root.clone(), laputa, move |name, value| {
                events.emit(name, value).map_err(|e| e.to_string())
            });
            app.manage(bridge);
            tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
                .data_directory(root.join("webview2"))
                .build()?;
            use tauri::{
                menu::{Menu, MenuItem},
                tray::TrayIconBuilder,
            };
            let open = MenuItem::with_id(app, "show", "Show Diva", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "exit", "Exit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &settings, &exit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show(app),
                    "settings" => {
                        show(app);
                        let _ = app.emit("open-settings", ());
                    }
                    "exit" => {
                        let app = app.clone();
                        let bridge = app.state::<Arc<Bridge>>().inner().clone();
                        std::thread::spawn(move || {
                            bridge.shutdown();
                            app.exit(0);
                        });
                    }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("failed to initialize Diva desktop");
    app.run(|app, event| {
        if let tauri::RunEvent::Exit = event {
            app.state::<Arc<Bridge>>().shutdown();
        }
    });
}
fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
