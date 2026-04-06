use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};
use tauri_plugin_store::StoreExt;

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

/// Read close_to_tray setting from the Tauri plugin store.
fn read_close_to_tray(app: &AppHandle<impl Runtime>) -> bool {
    match app.store("settings.json") {
        Ok(store) => store
            .get("closeToTray")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        Err(_) => false,
    }
}

/// Handle window close event based on close_to_tray setting.
/// Returns true if the app should hide (stay in tray), false if it should close.
pub fn handle_window_close<R: Runtime>(app: &AppHandle<R>) -> bool {
    if read_close_to_tray(app) {
        // Hide window instead of closing
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
        return true;
    }

    // Default behavior: don't close to tray, allow normal exit
    false
}
