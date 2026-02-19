use tauri::{Emitter, State};
use crate::app_state::AgentState;
use agent_diva_agent::AgentEvent;
use tokio::sync::mpsc;
use tracing::{info, error};

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
pub async fn send_message(
    message: String, 
    window: tauri::Window,
    state: State<'_, AgentState>
) -> Result<(), String> {
    info!("Received message from frontend: {}", message);
    
    let agent_arc = state.agent.clone();
    let window_arc = window.clone();
    
    // Check if agent is initialized
    {
        let agent = agent_arc.lock().await;
        if agent.is_none() {
            return Err("Agent is not configured. Please set API configuration.".to_string());
        }
    }
    
    // Spawn a task to run the agent loop for this message
    tauri::async_runtime::spawn(async move {
        let (tx, mut rx) = mpsc::unbounded_channel();
        
        // Spawn a task to listen to agent events and forward to frontend
        let forward_task = tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    AgentEvent::AssistantDelta { text } => {
                         let _ = window_arc.emit("agent-response-delta", text);
                    }
                    AgentEvent::ToolCallDelta { args_delta, .. } => {
                        // Forward tool call deltas as generic text for now, 
                        // but maybe we want a dedicated event for "thinking" or "tool-prep"
                        // For simplicity, let's treat it as a "tool-delta" event
                        let _ = window_arc.emit("agent-tool-delta", args_delta);
                    }
                    AgentEvent::FinalResponse { content } => {
                        let _ = window_arc.emit("agent-response-complete", content);
                    }
                    AgentEvent::ToolCallStarted { name, args_preview, .. } => {
                        let _ = window_arc.emit("agent-tool-start", format!("Using tool {}: {}", name, args_preview));
                    }
                    AgentEvent::ToolCallFinished { name, result, .. } => {
                        // Truncate result for display if too long
                        let display_result = if result.len() > 100 {
                            let mut end = 100;
                            while !result.is_char_boundary(end) {
                                end -= 1;
                            }
                            format!("{}...", &result[..end])
                        } else {
                            result
                        };
                        let _ = window_arc.emit("agent-tool-end", format!("Tool {} finished: {}", name, display_result));
                    }
                    AgentEvent::Error { message } => {
                        let _ = window_arc.emit("agent-error", message);
                    }
                    _ => {}
                }
            }
        });
        
        // Run the agent process
        let mut agent_lock = agent_arc.lock().await;
        
        if let Some(agent) = agent_lock.as_mut() {
             // We use a fixed session key "gui:main" for now
            let result = agent.process_direct_stream(
                message,
                "gui:main",
                "gui",
                "user",
                tx
            ).await;
            
            // Map error to string immediately so it's Send
            let result_string = result.map_err(|e| e.to_string());
            
            if let Err(e) = result_string {
                error!("Agent processing failed: {}", e);
                let _ = window.emit("agent-error", e);
            }
        } else {
             let _ = window.emit("agent-error", "Agent not configured".to_string());
        }
       
        
        // Wait for forwarding to finish
        let _ = forward_task.await;
    });
    
    Ok(())
}

#[tauri::command]
pub async fn update_config(
    api_base: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    state: State<'_, AgentState>
) -> Result<(), String> {
    state.reconfigure(api_base, api_key, model).await
}
