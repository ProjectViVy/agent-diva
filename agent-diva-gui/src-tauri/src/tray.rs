use agent_diva_core::config::ConfigLoader;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};
use tauri_plugin_store::StoreExt;

static TRAY_STATUS_ITEM: Mutex<Option<MenuItem<tauri::Wry>>> = Mutex::new(None);

/// Initialize the system tray with menu items.
pub fn init_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let status_text = app
        .try_state::<tokio::sync::Mutex<crate::gateway_status::GatewayStatus>>()
        .map(|state| {
            tauri::async_runtime::block_on(async {
                let status = state.lock().await;
                status.format_status()
            })
        })
        .unwrap_or_else(|| "Gateway: Unavailable".to_string());

    let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let status_item = MenuItem::with_id(app, "gateway_status", status_text, false, None::<&str>)?;
    let config_item = MenuItem::with_id(
        app,
        "open_config",
        "Open Config Directory",
        true,
        None::<&str>,
    )?;
    let logs_item = MenuItem::with_id(app, "open_logs", "Open Logs Directory", true, None::<&str>)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show_item,
            &separator,
            &status_item,
            &config_item,
            &logs_item,
            &separator2,
            &quit_item,
        ],
    )?;

    if let Ok(mut slot) = TRAY_STATUS_ITEM.lock() {
        *slot = Some(status_item.clone());
    }

    // Build the tray icon
    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id.as_ref() {
        "show" => show_main_window(app),
        "open_config" => open_config_directory(),
        "open_logs" => open_logs_directory(),
        "quit" => {
            crate::request_tray_quit(app.clone());
        }
        _ => {}
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn config_loader() -> ConfigLoader {
    match std::env::var("AGENT_DIVA_CONFIG_DIR") {
        Ok(path) if !path.trim().is_empty() => ConfigLoader::with_dir(expand_user_path(&path)),
        _ => ConfigLoader::new(),
    }
}

fn expand_user_path(path: &str) -> std::path::PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    std::path::PathBuf::from(path)
}

fn resolve_log_directory(loader: &ConfigLoader) -> std::path::PathBuf {
    let config = loader.load().unwrap_or_default();
    let path = expand_user_path(&config.logging.dir);
    if path.is_absolute() {
        path
    } else {
        loader.config_dir().join(path)
    }
}

fn open_config_directory() {
    let loader = config_loader();
    let path = loader.config_dir().to_path_buf();
    open_directory(&path, "config");
}

fn open_logs_directory() {
    let loader = config_loader();
    let path = resolve_log_directory(&loader);
    open_directory(&path, "logs");
}

fn open_directory(path: &std::path::Path, label: &str) {
    if !path.exists() {
        tracing::warn!("{} directory does not exist: {}", label, path.display());
        return;
    }

    if let Err(error) = open::that(path) {
        tracing::error!(
            "Failed to open {} directory {}: {}",
            label,
            path.display(),
            error
        );
    }
}

pub fn update_tray_status(status_text: &str) {
    match TRAY_STATUS_ITEM.lock() {
        Ok(slot) => {
            if let Some(item) = slot.as_ref() {
                if let Err(error) = item.set_text(status_text) {
                    tracing::warn!("Failed to update tray status text: {}", error);
                }
            }
        }
        Err(error) => {
            tracing::warn!("Failed to lock tray status item: {}", error);
        }
    }
}

/// Read close_to_tray setting from the Tauri plugin store.
pub fn read_close_to_tray(app: &AppHandle) -> bool {
    match app.store("settings.json") {
        Ok(store) => store
            .get("closeToTray")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        Err(_) => false,
    }
}

pub fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}
