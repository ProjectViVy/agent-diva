use std::path::PathBuf;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

/// Initialize the system tray with menu items.
pub fn init_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    // Create tray menu items
    let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    // Build the tray icon
    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                // User explicitly chose to quit from tray menu
                app.exit(0);
            }
            _ => {}
        })
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

/// Read close_to_tray setting from the settings.json file
fn read_close_to_tray_from_file(app: &AppHandle<impl Runtime>) -> Option<bool> {
    let config_dir = app.path().app_config_dir().ok()?;
    let settings_path: PathBuf = config_dir.join("settings.json");

    let content = std::fs::read_to_string(&settings_path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    json.get("closeToTray")?.as_bool()
}

/// Handle window close event based on close_to_tray setting.
/// Returns true if the app should hide (stay in tray), false if it should close.
pub fn handle_window_close<R: Runtime>(app: &AppHandle<R>) -> bool {
    // Try to read the close_to_tray setting from the file
    if let Some(close_to_tray) = read_close_to_tray_from_file(app) {
        if close_to_tray {
            // Hide window instead of closing
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
            return true;
        }
    }

    // Default behavior: don't close to tray, allow normal exit
    false
}
