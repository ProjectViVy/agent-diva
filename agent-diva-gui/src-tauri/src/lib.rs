// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod app_state;
mod commands;

use app_state::AgentState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AgentState::new())
        .setup(|_app| Ok(()))
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::send_message,
            commands::stop_generation,
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
