// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod app_state;
mod commands;
mod embedded_server;
mod gateway_status;
mod notebook;
mod process_utils;
mod shutdown_manager;
mod tray;

use agent_diva_core::config::schema::{Config, LoggingConfig};
use agent_diva_core::config::ConfigLoader;
use app_state::AgentState;
use embedded_server::EmbeddedGatewayHandle;
use gateway_status::GatewayStatus;
use shutdown_manager::ShutdownManager;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tokio::sync::Mutex as AsyncMutex;

static LOGGING_INITIALIZED: OnceLock<()> = OnceLock::new();
const EXTERNAL_GATEWAY_ENV: &str = "AGENT_DIVA_EXTERNAL_GATEWAY";

/// Tracks completion of frontend/backend setup for splash screen.
struct SplashState {
    frontend_done: bool,
    backend_done: bool,
}

pub type EmbeddedGatewayState = Arc<AsyncMutex<Option<EmbeddedGatewayHandle>>>;
pub type WorkspaceSwitchState = Arc<AsyncMutex<()>>;

fn should_manage_gateway_lifecycle() -> bool {
    should_manage_gateway_lifecycle_from(std::env::var(EXTERNAL_GATEWAY_ENV).ok().as_deref())
}

fn should_manage_gateway_lifecycle_from(external_gateway: Option<&str>) -> bool {
    !external_gateway.is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn config_loader() -> ConfigLoader {
    match std::env::var("AGENT_DIVA_CONFIG_DIR") {
        Ok(path) if !path.trim().is_empty() => ConfigLoader::with_dir(expand_user_path(&path)),
        _ => ConfigLoader::new(),
    }
}

fn expand_user_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn resolve_configured_path(path: &str, config_dir: &Path) -> PathBuf {
    let expanded = expand_user_path(path);
    if expanded.is_absolute() {
        expanded
    } else {
        config_dir.join(expanded)
    }
}

fn resolve_logging_config(mut config: LoggingConfig, config_dir: &Path) -> LoggingConfig {
    config.dir = resolve_configured_path(&config.dir, config_dir)
        .to_string_lossy()
        .to_string();
    config
}

fn init_gui_logging() {
    LOGGING_INITIALIZED.get_or_init(|| {
        let loader = config_loader();
        let config = loader.load().unwrap_or_default();
        let logging = resolve_logging_config(config.logging, loader.config_dir());
        let guard = agent_diva_core::logging::init_logging_with_terminal_output(&logging, false);
        Box::leak(Box::new(guard));
    });
}

pub(crate) fn build_gateway_runtime_config() -> agent_diva_manager::GatewayRuntimeConfig {
    let loader = config_loader();
    let mut config = loader.load().unwrap_or_default();
    normalize_gui_default_workspace(&mut config, loader.config_dir());
    let runtime = agent_diva_cli::cli_runtime::CliRuntime::from_paths(
        None,
        Some(loader.config_dir().to_path_buf()),
        None,
    );

    agent_diva_manager::GatewayRuntimeConfig {
        workspace: runtime.workspace_context(&config),
        cron_store: runtime.cron_store_path(),
        config,
        loader,
        port: 0,
    }
}

fn normalize_gui_default_workspace(config: &mut Config, config_dir: &Path) {
    if config.agents.defaults.workspace != agent_diva_core::workspace::LEGACY_DEFAULT_WORKSPACE {
        return;
    }
    let root = config_dir.join("workspace");
    if let Err(error) = std::fs::create_dir_all(&root) {
        tracing::warn!(
            path = %root.display(),
            %error,
            "failed to create the built-in Diva default workspace"
        );
    }
    config.agents.defaults.workspace = root.display().to_string();
}

pub(crate) fn build_gateway_runtime_config_for_workspace(
    root: &Path,
    source: agent_diva_core::workspace::WorkspaceSource,
) -> agent_diva_manager::GatewayRuntimeConfig {
    let mut runtime = build_gateway_runtime_config();
    runtime.workspace = workspace_runtime_override(root, source);
    runtime
}

fn workspace_runtime_override(
    root: &Path,
    source: agent_diva_core::workspace::WorkspaceSource,
) -> agent_diva_core::workspace::WorkspaceContext {
    agent_diva_core::workspace::WorkspaceContext {
        root: root.to_path_buf(),
        source,
        agents_md: None,
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

fn close_all_webview_windows_for_exit(app: &tauri::AppHandle) {
    let _ = app.emit_to("desktop-mate", "desktop-mate-render-pause", true);

    if let Some(window) = app.get_webview_window("desktop-mate") {
        let _ = window.set_ignore_cursor_events(false);
    }

    for (label, window) in app.webview_windows() {
        tracing::debug!("Closing webview window '{label}' before app exit");
        let _ = window.hide();
        let _ = window.close();
    }
}

pub fn request_full_exit(app: tauri::AppHandle) {
    let Some(shutdown_manager) = app.try_state::<ShutdownManager>() else {
        tracing::warn!("Shutdown manager unavailable; exiting application directly");
        close_all_webview_windows_for_exit(&app);
        app.exit(0);
        return;
    };

    if !shutdown_manager.begin_shutdown() {
        tracing::debug!("Full exit already in progress");
        return;
    }

    tracing::info!("Starting full application shutdown");
    close_all_webview_windows_for_exit(&app);

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
    init_gui_logging();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(AgentState::new())
        .manage(ShutdownManager::new())
        .manage(Arc::new(Mutex::new(SplashState {
            frontend_done: false,
            backend_done: false,
        })))
        .manage(Arc::new(AsyncMutex::new(())) as WorkspaceSwitchState)
        .setup(|app| {
            let (gateway_state, gateway_port) = if should_manage_gateway_lifecycle() {
                let handle = embedded_server::start_embedded_gateway(build_gateway_runtime_config())
                    .map_err(|error| {
                        tracing::error!("Failed to start embedded gateway: {}", error);
                        std::io::Error::other(format!(
                            "failed to start embedded gateway: {error}"
                        ))
                    })?;
                let port = handle.port;
                (Arc::new(AsyncMutex::new(Some(handle))), Some(port))
            } else {
                tracing::info!(
                    env = EXTERNAL_GATEWAY_ENV,
                    "Embedded gateway lifecycle is disabled explicitly; expecting an external backend"
                );
                (Arc::new(AsyncMutex::new(None)), None)
            };

            app.manage(gateway_state);
            let status = gateway_port
                .map(GatewayStatus::new)
                .unwrap_or_else(|| GatewayStatus::stopped(3000));
            app.manage(AsyncMutex::new(status));
            if let Some(port) = gateway_port {
                app.state::<AgentState>().update_gateway_port(port);
                commands::save_gateway_port_config(port)
                    .map_err(std::io::Error::other)?;
                tracing::info!("Embedded gateway started on port {}", port);
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
                if window
                    .app_handle()
                    .try_state::<ShutdownManager>()
                    .map(|manager| manager.is_shutting_down())
                    .unwrap_or(false)
                {
                    tracing::debug!(
                        "Allowing window '{}' to close because shutdown is already in progress",
                        window.label()
                    );
                    return;
                }

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
            commands::continue_approved_plan_execution,
            commands::stop_generation,
            commands::reset_session,
            commands::get_sessions,
            commands::get_session_history,
            commands::delete_session,
            commands::update_session_title,
            commands::generate_session_title,
            commands::get_cron_jobs,
            commands::get_cron_job,
            commands::get_plans,
            commands::get_plan,
            commands::delete_plan,
            commands::delete_plan_todo,
            commands::restore_plan_todo,
            commands::get_active_plan,
            commands::approve_active_plan_execution,
            commands::return_active_plan_to_draft,
            commands::get_plan_reports,
            commands::approve_plan_report,
            commands::get_active_plan_execution,
            commands::get_execution_todos,
            commands::update_execution_todo,
            commands::create_cron_job,
            commands::update_cron_job,
            commands::set_cron_job_enabled,
            commands::run_cron_job,
            commands::stop_cron_job_run,
            commands::delete_cron_job,
            commands::start_background_stream,
            commands::get_command_approvals,
            commands::resolve_command_approval,
            commands::list_approvals,
            commands::get_approval,
            commands::decide_approval,
            commands::cancel_approval,
            commands::start_approval_stream,
            commands::list_ask_user_questions,
            commands::answer_ask_user_question,
            commands::cancel_ask_user_question,
            commands::get_command_rules,
            commands::set_command_rule_enabled,
            commands::delete_command_rule,
            commands::update_config,
            commands::get_tools_config,
            commands::update_tools_config,
            commands::get_skills,
            commands::get_skill,
            commands::update_skill,
            commands::disable_skill,
            commands::list_skill_history,
            commands::get_skill_history_revision,
            commands::list_skill_requests,
            commands::create_skill_request,
            commands::get_skill_request,
            commands::accept_skill_request,
            commands::reject_skill_request,
            commands::list_masks,
            commands::get_current_mask,
            commands::get_active_mask,
            commands::switch_mask,
            commands::create_or_update_mask,
            commands::delete_mask,
            commands::get_mcps,
            commands::create_mcp,
            commands::update_mcp,
            commands::delete_mcp,
            commands::set_mcp_enabled,
            commands::refresh_mcp_status,
            commands::upload_skill,
            commands::search_marketplace_skills,
            commands::install_marketplace_skill,
            commands::featured_marketplace_skills,
            commands::upload_file,
            commands::delete_skill,
            commands::get_providers,
            commands::create_custom_provider,
            commands::delete_custom_provider,
            commands::add_provider_model,
            commands::remove_provider_model,
            commands::get_provider_models,
            commands::test_provider_model,
            commands::persona_get_status,
            commands::persona_initialize,
            commands::persona_repair,
            commands::persona_get_document,
            commands::persona_save_document,
            commands::persona_list_history,
            commands::persona_get_history_revision,
            commands::persona_list_requests,
            commands::persona_accept_request,
            commands::persona_reject_request,
            commands::memory_list_records,
            commands::memory_create_record,
            commands::memory_get_record,
            commands::memory_update_record,
            commands::memory_delete_record,
            commands::memory_get_actmem,
            commands::memory_put_actmem,
            commands::memory_list_capsules,
            commands::memory_get_capsule,
            commands::memory_delete_capsule,
            commands::memory_get_memrules,
            commands::memory_put_memrules,
            commands::trigger_autodream,
            commands::get_notebook_reports,
            commands::trigger_notebook_report_generation,
            commands::search_notebook_session_evidence_command,
            commands::get_autodream_run_status,
            commands::list_autodream_run_events,
            commands::get_autodream_live_text,
            commands::cancel_autodream_run,
            commands::list_autodream_run_records,
            commands::list_recall_feedback,
            commands::get_self_evolution_config,
            commands::save_self_evolution_config,
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
            commands::get_workspace_status,
            commands::inspect_workspace,
            commands::choose_workspace_directory,
            commands::switch_workspace,
            commands::get_default_workspace,
            commands::set_default_workspace,
            commands::reset_default_workspace,
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
            commands::get_token_usage_total,
            commands::get_token_usage_summary,
            commands::get_token_usage_timeline,
            commands::get_token_usage_sessions,
            commands::get_token_usage_models,
            commands::get_token_usage_realtime,
            commands::get_sandbox_config,
            commands::save_sandbox_config,
            commands::get_gui_prefs,
            commands::set_gui_prefs,
            commands::mate_list_vrm_models,
            commands::mate_import_vrm_model,
            commands::mate_delete_vrm_model,
            commands::mate_read_vrm_model,
            commands::mate_load_voice_assets,
            commands::mate_save_voice_selection,
            commands::mate_import_voice_file,
            commands::mate_delete_voice_file,
            commands::mate_read_voice_file,
            commands::mate_minimax_synthesize,
            commands::mate_siliconflow_synthesize,
            commands::open_desktop_mate,
            commands::close_desktop_mate,
            commands::set_desktop_mate_ignore_mouse,
            commands::set_desktop_mate_always_on_top,
            commands::minimize_desktop_mate,
            commands::get_audit_events,
            commands::get_gateway_log_lines,
            commands::append_gui_log,
            commands::get_gui_log_lines
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
    use super::{
        close_action, normalize_gui_default_workspace, resolve_configured_path,
        resolve_logging_config, should_manage_gateway_lifecycle_from, workspace_runtime_override,
        CloseAction, ExitTrigger,
    };
    use crate::shutdown_manager::ShutdownManager;
    use agent_diva_core::config::schema::LoggingConfig;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn gateway_lifecycle_is_managed_by_default() {
        assert!(should_manage_gateway_lifecycle_from(None));
        assert!(should_manage_gateway_lifecycle_from(Some("")));
        assert!(should_manage_gateway_lifecycle_from(Some("false")));
        assert!(should_manage_gateway_lifecycle_from(Some("0")));
    }

    #[test]
    fn external_gateway_mode_requires_an_explicit_truthy_flag() {
        for value in ["1", "true", "TRUE", " yes ", "on"] {
            assert!(
                !should_manage_gateway_lifecycle_from(Some(value)),
                "{value:?} should enable external gateway mode"
            );
        }
    }

    #[test]
    fn legacy_gui_default_is_normalized_to_the_profile_workspace() {
        let temp = TempDir::new().unwrap();
        let mut config = agent_diva_core::config::schema::Config::default();

        normalize_gui_default_workspace(&mut config, temp.path());

        assert_eq!(
            config.agents.defaults.workspace,
            temp.path().join("workspace").display().to_string()
        );
        assert!(temp.path().join("workspace").is_dir());
    }

    #[test]
    fn workspace_runtime_override_preserves_the_requested_source() {
        let root = PathBuf::from("session-workspace");

        let workspace = workspace_runtime_override(
            &root,
            agent_diva_core::workspace::WorkspaceSource::ExplicitCli,
        );

        assert_eq!(workspace.root, root);
        assert_eq!(
            workspace.source,
            agent_diva_core::workspace::WorkspaceSource::ExplicitCli
        );
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

    #[test]
    fn relative_log_dir_resolves_under_config_dir() {
        let config_dir = TempDir::new().unwrap();

        let path = resolve_configured_path("logs", config_dir.path());

        assert_eq!(path, config_dir.path().join("logs"));
    }

    #[test]
    fn absolute_log_dir_is_preserved() {
        let config_dir = TempDir::new().unwrap();
        let absolute = config_dir.path().join("custom-logs");

        let path = resolve_configured_path(&absolute.to_string_lossy(), config_dir.path());

        assert_eq!(path, absolute);
    }

    #[test]
    fn logging_config_uses_resolved_log_dir() {
        let config_dir = TempDir::new().unwrap();
        let logging = LoggingConfig {
            dir: "logs".to_string(),
            ..LoggingConfig::default()
        };

        let resolved = resolve_logging_config(logging, config_dir.path());

        assert_eq!(
            resolved.dir,
            config_dir.path().join("logs").to_string_lossy().to_string()
        );
    }
}
