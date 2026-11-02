mod bridge;
mod diagnostics;
mod process;
mod storage;
use bridge::Bridge;
use serde_json::Value;
use std::sync::Arc;
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
            core_diagnostics
        ])
        .setup(|app| {
            let root = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(root.join("webview2"))?;
            let events = app.handle().clone();
            let bridge = Bridge::new(root.clone(), move |name, value| {
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
