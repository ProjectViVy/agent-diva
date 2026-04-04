// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod app_state;
mod commands;
mod process_utils;
mod tray;

use app_state::AgentState;
use std::sync::{Arc, Mutex};
use tauri::async_runtime::spawn;
use tauri::Manager;

/// Tracks completion of frontend/backend setup for splash screen.
struct SplashState {
    frontend_done: bool,
    backend_done: bool,
}

fn should_manage_gateway_lifecycle() -> bool {
    !cfg!(debug_assertions)
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
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(AgentState::new())
        .manage(Arc::new(Mutex::new(SplashState {
            frontend_done: false,
            backend_done: false,
        })))
        .setup(|app| {
            if should_manage_gateway_lifecycle() {
                // Cleanup orphan gateway processes on startup
                let cleanup_result = process_utils::cleanup_orphan_gateway_processes();
                match cleanup_result {
                    Ok(count) if count > 0 => {
                        tracing::info!("Cleaned up {} orphan gateway process(es) on startup", count);
                    }
                    Ok(_) => {
                        tracing::debug!("No orphan gateway processes found on startup");
                    }
                    Err(e) => {
                        tracing::warn!("Failed to cleanup orphan processes on startup: {}", e);
                    }
                }

                // Auto-start gateway on GUI launch
                let app_handle = app.handle().clone();
                spawn(async move {
                    // Wait a bit for GUI to initialize
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                    tracing::info!("Auto-starting gateway...");
                    match commands::start_gateway(app_handle.clone(), None).await {
                        Ok(port) => {
                            tracing::info!("Gateway auto-started successfully on port {}", port);
                        }
                        Err(e) => {
                            tracing::error!("Failed to auto-start gateway: {}", e);
                        }
                    }
                });
            } else {
                tracing::info!(
                    "Gateway lifecycle management is disabled in debug mode; expecting an external backend"
                );
            }

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

            // Initialize system tray
            if let Err(e) = tray::init_tray(app.handle()) {
                tracing::warn!("Failed to initialize system tray: {}", e);
            } else {
                tracing::info!("System tray initialized successfully");
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Handle window close events based on tray setting
            if let tauri::WindowEvent::CloseRequested { api, .. } = &event {
                let window_label = window.label();
                if window_label == "main" {
                    // Check if we should hide to tray instead of closing
                    if tray::handle_window_close(window.app_handle()) {
                        // Prevent window close, it was already hidden by handle_window_close
                        api.prevent_close();
                        tracing::info!("Window hidden to system tray");
                        return;
                    }

                    // Normal close - cleanup gateway if needed
                    if should_manage_gateway_lifecycle() {
                        tracing::info!("Main window closing, cleaning up gateway process...");
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = commands::stop_gateway().await {
                                tracing::error!("Failed to stop gateway on exit: {}", e);
                            } else {
                                tracing::info!("Gateway process stopped successfully on exit");
                            }
                        });
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            set_splash_complete,
            commands::greet,
            commands::send_message,
            commands::stop_generation,
            commands::reset_session,
            commands::get_sessions,
            commands::get_session_history,
            commands::delete_session,
            commands::get_cron_jobs,
            commands::get_cron_job,
            commands::create_cron_job,
            commands::update_cron_job,
            commands::set_cron_job_enabled,
            commands::run_cron_job,
            commands::stop_cron_job_run,
            commands::delete_cron_job,
            commands::start_background_stream,
            commands::update_config,
            commands::get_tools_config,
            commands::update_tools_config,
            commands::get_skills,
            commands::get_mcps,
            commands::create_mcp,
            commands::update_mcp,
            commands::delete_mcp,
            commands::set_mcp_enabled,
            commands::refresh_mcp_status,
            commands::upload_skill,
            commands::delete_skill,
            commands::get_providers,
            commands::create_custom_provider,
            commands::delete_custom_provider,
            commands::add_provider_model,
            commands::remove_provider_model,
            commands::get_provider_models,
            commands::test_provider_model,
            commands::get_channels,
            commands::update_channel,
            commands::check_health,
            commands::get_gateway_process_status,
            commands::start_gateway,
            commands::stop_gateway,
            commands::uninstall_gateway,
            commands::load_config,
            commands::get_config,
            commands::get_config_status,
            commands::wipe_local_data,
            commands::save_config,
            commands::tail_logs,
            commands::get_runtime_info,
            commands::get_service_status,
            commands::install_service,
            commands::uninstall_service,
            commands::start_service,
            commands::stop_service,
            commands::get_gui_prefs,
            commands::set_gui_prefs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::should_manage_gateway_lifecycle;

    #[test]
    fn gateway_lifecycle_is_disabled_in_debug_builds() {
        assert_eq!(should_manage_gateway_lifecycle(), !cfg!(debug_assertions));
    }
}
