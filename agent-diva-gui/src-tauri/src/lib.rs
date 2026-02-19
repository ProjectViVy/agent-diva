// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod app_state;
mod commands;
mod server;

use tauri::Manager;
use app_state::AgentState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AgentState::new())
        .setup(|app| {
            // Start the HTTP Hook server
            let app_handle = app.handle().clone();
            let port = 3000;
            
            // Spawn the server in a separate async task
            tauri::async_runtime::spawn(async move {
                // Wait for the runtime to be fully ready if needed, 
                // but usually spawn is enough.
                if let Err(e) = server::start_server(app_handle, port).await {
                    eprintln!("Failed to start HTTP Hook server: {}", e);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::send_message,
            commands::update_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
