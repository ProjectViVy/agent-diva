// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod app_state;
mod commands;
mod embedded_server;
mod gateway_status;
mod process_utils;
mod shutdown_manager;
mod tray;

use app_state::AgentState;
use embedded_server::EmbeddedGatewayHandle;
use gateway_status::GatewayStatus;
use shutdown_manager::ShutdownManager;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tokio::sync::Mutex as AsyncMutex;

/// Tracks completion of frontend/backend setup for splash screen.
struct SplashState {
    frontend_done: bool,
    backend_done: bool,
}

pub type EmbeddedGatewayState = Arc<AsyncMutex<Option<EmbeddedGatewayHandle>>>;

fn should_manage_gateway_lifecycle() -> bool {
    // Only manage gateway lifecycle in release mode
    // In debug mode, developers should start the gateway manually for better control
    !cfg!(debug_assertions)
}

fn build_gateway_runtime_config() -> agent_diva_manager::GatewayRuntimeConfig {
    let loader = agent_diva_core::config::ConfigLoader::new();
    let config = loader.load().unwrap_or_default();
    let runtime = agent_diva_cli::cli_runtime::CliRuntime::from_paths(
        None,
        Some(loader.config_dir().to_path_buf()),
        None,
    );

    agent_diva_manager::GatewayRuntimeConfig {
        workspace: runtime.effective_workspace(&config),
        cron_store: runtime.cron_store_path(),
        config,
        loader,
        port: 0,
    }
}

pub async fn shutdown_embedded_gateway(app: &tauri::AppHandle) {
    if should_manage_gateway_lifecycle() {
        if let Some(status) = app.try_state::<AsyncMutex<GatewayStatus>>() {
            let mut guard = status.lock().await;
            guard.stop();
            tray::update_tray_status(&guard.format_status());
        }

        if let Some(state) = app.try_state::<EmbeddedGatewayState>() {
            let mut guard = state.lock().await;
            if let Some(handle) = guard.take() {
                handle.shutdown();
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExitTrigger {
    MainWindowClose,
    TrayQuit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloseAction {
    HideToTray,
    FullExit,
}

fn close_action(close_to_tray: bool, trigger: ExitTrigger) -> CloseAction {
    match trigger {
        ExitTrigger::MainWindowClose if close_to_tray => CloseAction::HideToTray,
        ExitTrigger::MainWindowClose | ExitTrigger::TrayQuit => CloseAction::FullExit,
    }
}

pub fn request_full_exit(app: tauri::AppHandle) {
    let Some(shutdown_manager) = app.try_state::<ShutdownManager>() else {
        tracing::warn!("Shutdown manager unavailable; exiting application directly");
        app.exit(0);
        return;
    };

    if !shutdown_manager.begin_shutdown() {
        tracing::debug!("Full exit already in progress");
        return;
    }

    tracing::info!("Starting full application shutdown");
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }

    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;

        let shutdown_result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            shutdown_embedded_gateway(&app),
        )
        .await;

        match shutdown_result {
            Ok(_) => tracing::info!("Embedded gateway shutdown completed before process exit"),
            Err(_) => tracing::warn!(
                "Embedded gateway shutdown timed out after 5 seconds; continuing app exit"
            ),
        }

        app.exit(0);
    });
}

pub fn request_tray_quit(app: tauri::AppHandle) {
    if close_action(tray::read_close_to_tray(&app), ExitTrigger::TrayQuit) == CloseAction::FullExit
    {
        request_full_exit(app);
    }
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
        .manage(ShutdownManager::new())
        .manage(Arc::new(Mutex::new(SplashState {
            frontend_done: false,
            backend_done: false,
        })))
        .setup(|app| {
            if should_manage_gateway_lifecycle() {
                let handle = embedded_server::start_embedded_gateway(build_gateway_runtime_config())
                    .map_err(|error| {
                        tracing::error!("Failed to start embedded gateway: {}", error);
                        std::io::Error::other(format!(
                            "failed to start embedded gateway: {error}"
                        ))
                    })?;
                let port = handle.port;
                let gateway_state: EmbeddedGatewayState = Arc::new(AsyncMutex::new(Some(handle)));

                app.manage(gateway_state);
                app.manage(AsyncMutex::new(GatewayStatus::new(port)));
                app.state::<AgentState>().update_gateway_port(port);
                commands::save_gateway_port_config(port)
                    .map_err(std::io::Error::other)?;
                tracing::info!("Embedded gateway started on port {}", port);
            } else {
                tracing::info!(
                    "Gateway lifecycle management is disabled in debug mode; expecting an external backend"
                );
            }

            if let Ok(mut guard) = app.state::<Arc<Mutex<SplashState>>>().lock() {
                guard.backend_done = true;
                if guard.frontend_done {
                    drop(guard);
                    if let Some(splash) = app.get_webview_window("splashscreen") {
                        let _ = splash.close();
                    }
                    if let Some(main_win) = app.get_webview_window("main") {
                        let _ = main_win.show();
                        let _ = main_win.set_focus();
                    }
                }
            }

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
                    let app_handle = window.app_handle().clone();
                    let action = close_action(
                        tray::read_close_to_tray(&app_handle),
                        ExitTrigger::MainWindowClose,
                    );
                    match action {
                        CloseAction::HideToTray => {
                            api.prevent_close();
                            tray::hide_main_window(&app_handle);
                            tracing::info!("Window hidden to system tray");
                        }
                        CloseAction::FullExit => {
                            api.prevent_close();
                            tracing::info!("Main window close requested; performing full exit");
                            request_full_exit(app_handle);
                        }
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
            commands::upload_file,
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
            commands::get_gateway_status,
            commands::get_gateway_process_status,
            #[allow(deprecated)]
            commands::start_gateway,
            #[allow(deprecated)]
            commands::stop_gateway,
            #[allow(deprecated)]
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
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(shutdown_manager) = app.try_state::<ShutdownManager>() {
                    let already_shutting_down = shutdown_manager.is_shutting_down();
                    shutdown_manager.mark_exit_observed();
                    if already_shutting_down {
                        tracing::debug!("RunEvent::Exit observed after shutdown already started");
                    }
                }
                tracing::info!("Application exiting, shutting down embedded gateway");
                tauri::async_runtime::block_on(shutdown_embedded_gateway(app));
            }
        })
}

#[cfg(test)]
mod tests {
    use super::{close_action, should_manage_gateway_lifecycle, CloseAction, ExitTrigger};
    use crate::shutdown_manager::ShutdownManager;

    #[test]
    fn gateway_lifecycle_is_disabled_in_debug_mode() {
        // In debug mode, gateway lifecycle should be disabled
        // In release mode, gateway lifecycle should be enabled
        if cfg!(debug_assertions) {
            assert!(
                !should_manage_gateway_lifecycle(),
                "Gateway lifecycle should be disabled in debug mode"
            );
        } else {
            assert!(
                should_manage_gateway_lifecycle(),
                "Gateway lifecycle should be enabled in release mode"
            );
        }
    }

    #[test]
    fn main_window_close_hides_to_tray_when_enabled() {
        assert_eq!(
            close_action(true, ExitTrigger::MainWindowClose),
            CloseAction::HideToTray
        );
    }

    #[test]
    fn main_window_close_exits_when_background_residency_disabled() {
        assert_eq!(
            close_action(false, ExitTrigger::MainWindowClose),
            CloseAction::FullExit
        );
    }

    #[test]
    fn tray_quit_always_exits_even_when_background_residency_enabled() {
        assert_eq!(
            close_action(true, ExitTrigger::TrayQuit),
            CloseAction::FullExit
        );
    }

    #[test]
    fn shutdown_manager_is_idempotent() {
        let manager = ShutdownManager::new();

        assert!(manager.begin_shutdown());
        assert!(!manager.begin_shutdown());
    }
}
