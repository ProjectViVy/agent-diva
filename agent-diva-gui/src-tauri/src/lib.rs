// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod app_state;
mod commands;

use tauri::Manager;
use app_state::AgentState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AgentState::new())
        .setup(|app| {
            // Setup complete
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::send_message,
            commands::update_config,
            commands::get_providers,
            commands::get_channels,
            commands::update_channel
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
