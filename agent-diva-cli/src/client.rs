use anyhow::Result;
use futures::StreamExt;
use reqwest::Client;
use eventsource_stream::Eventsource;
use tokio::sync::mpsc;
use agent_diva_agent::AgentEvent;
use serde::Deserialize;

pub struct ApiClient {
    client: Client,
    base_url: String,
}

#[derive(Deserialize)]
struct ToolStartEvent {
    name: String,
    #[serde(alias = "args")]
    args_preview: String,
    id: String,
}

#[derive(Deserialize)]
struct ToolFinishEvent {
    name: String,
    result: String,
    error: bool,
    id: String,
}

#[derive(Deserialize)]
struct ToolDeltaEvent {
    name: String,
    delta: String,
}

impl ApiClient {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "http://localhost:3000/api".to_string()),
        }
    }

    pub async fn chat(&self, message: String, event_tx: mpsc::UnboundedSender<AgentEvent>) -> Result<()> {
        let url = format!("{}/chat", self.base_url);
        let response = self.client.post(&url)
            .json(&serde_json::json!({ "message": message }))
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Server returned error: {}", response.status());
        }

        let mut stream = response.bytes_stream().eventsource();

        while let Some(event) = stream.next().await {
            match event {
                Ok(event) => {
                    match event.event.as_str() {
                        "delta" => {
                            let _ = event_tx.send(AgentEvent::AssistantDelta { text: event.data });
                        }
                        "final" => {
                            let _ = event_tx.send(AgentEvent::FinalResponse { content: event.data });
                        }
                        "tool_start" => {
                            if let Ok(data) = serde_json::from_str::<ToolStartEvent>(&event.data) {
                                let _ = event_tx.send(AgentEvent::ToolCallStarted { 
                                    name: data.name, 
                                    args_preview: data.args_preview, 
                                    call_id: data.id 
                                });
                            }
                        }
                        "tool_finish" => {
                            if let Ok(data) = serde_json::from_str::<ToolFinishEvent>(&event.data) {
                                let _ = event_tx.send(AgentEvent::ToolCallFinished { 
                                    name: data.name, 
                                    result: data.result, 
                                    is_error: data.error, 
                                    call_id: data.id 
                                });
                            }
                        }
                        "tool_delta" => {
                             if let Ok(data) = serde_json::from_str::<ToolDeltaEvent>(&event.data) {
                                let _ = event_tx.send(AgentEvent::ToolCallDelta { 
                                    name: Some(data.name), 
                                    args_delta: data.delta,
                                });
                            }
                        }
                        "error" => {
                            let _ = event_tx.send(AgentEvent::Error { message: event.data });
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    let _ = event_tx.send(AgentEvent::Error { message: e.to_string() });
                }
            }
        }
        Ok(())
    }
}
