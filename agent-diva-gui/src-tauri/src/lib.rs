// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod app_state;
mod commands;

use app_state::AgentState;
use std::sync::{Arc, Mutex};
use tauri::async_runtime::spawn;
use tauri::Manager;

/// Tracks completion of frontend/backend setup for splash screen.
struct SplashState {
    frontend_done: bool,
    backend_done: bool,
}

#[tauri::command]
fn set_splash_complete(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Mutex<SplashState>>>,
    task: String,
) -> Result<(), String> {
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    match task.as_str() {
        "frontend" => guard.frontend_done = true,
        "backend" => guard.backend_done = true,
        _ => return Err(format!("invalid task: {}", task)),
    }

    if guard.frontend_done && guard.backend_done {
        drop(guard);
        if let Some(splash) = app.get_webview_window("splashscreen") {
            let _ = splash.close();
        }
        if let Some(main_win) = app.get_webview_window("main") {
            let _ = main_win.show();
            let _ = main_win.set_focus();
        }
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AgentState::new())
        .manage(Arc::new(Mutex::new(SplashState {
            frontend_done: false,
            backend_done: false,
        })))
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<Arc<Mutex<SplashState>>>().inner().clone();
            spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                if let Ok(mut guard) = state.lock() {
                    guard.backend_done = true;
                    if guard.frontend_done {
                        drop(guard);
                        if let Some(splash) = handle.get_webview_window("splashscreen") {
                            let _ = splash.close();
                        }
                        if let Some(main_win) = handle.get_webview_window("main") {
                            let _ = main_win.show();
                            let _ = main_win.set_focus();
                        }
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_splash_complete,
            commands::greet,
            commands::send_message,
            commands::stop_generation,
            commands::reset_session,
            commands::get_sessions,
            commands::get_session_history,
            commands::start_background_stream,
            commands::update_config,
            commands::get_tools_config,
            commands::update_tools_config,
            commands::get_providers,
            commands::get_channels,
            commands::update_channel,
            commands::test_channel,
            commands::check_health
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
