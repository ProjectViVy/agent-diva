use crate::approval_commands::{ApprovalListPage, ApprovalView, APPROVAL_QUEUE_UNAVAILABLE};
use agent_diva_agent::AgentEvent;
use agent_diva_core::planning::update_plan::UpdatePlanArgs;
use anyhow::Result;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;

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

    pub async fn list_approvals(
        &self,
        status: Option<&str>,
        session: Option<&str>,
    ) -> Result<ApprovalListPage> {
        let url = format!("{}/approvals", self.base_url);
        let mut approvals = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..10 {
            let mut request = self.client.get(&url).query(&[("limit", "100")]);
            if let Some(status) = status {
                request = request.query(&[("status", status)]);
            }
            if let Some(session) = session {
                request = request.query(&[("session", session)]);
            }
            if let Some(cursor) = &cursor {
                request = request.query(&[("cursor", cursor)]);
            }
            let response = request
                .send()
                .await
                .map_err(|_| anyhow::anyhow!(APPROVAL_QUEUE_UNAVAILABLE))?;
            let page: ApprovalListPage = parse_response(response).await?;
            approvals.extend(page.approvals);
            let Some(next) = page.next_cursor else {
                return Ok(ApprovalListPage {
                    approvals,
                    next_cursor: None,
                });
            };
            cursor = Some(next);
        }
        Ok(ApprovalListPage {
            approvals,
            next_cursor: cursor,
        })
    }

    pub async fn get_approval(&self, request_id: &str) -> Result<ApprovalView> {
        let url = format!("{}/approvals/{request_id}", self.base_url);
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_| anyhow::anyhow!(APPROVAL_QUEUE_UNAVAILABLE))?;
        parse_response(response).await
    }

    pub async fn decide_approval(
        &self,
        request_id: &str,
        expected_version: u64,
        idempotency_key: &str,
        decision: &str,
        grant: &str,
    ) -> Result<ApprovalView> {
        let url = format!("{}/approvals/{request_id}/decisions", self.base_url);
        parse_response(
            self.client
                .post(url)
                .json(&serde_json::json!({
                    "expected_version": expected_version,
                    "idempotency_key": idempotency_key,
                    "decision": decision,
                    "grant": grant,
                }))
                .send()
                .await
                .map_err(|_| anyhow::anyhow!(APPROVAL_QUEUE_UNAVAILABLE))?,
        )
        .await
    }

    pub async fn cancel_approval(
        &self,
        request_id: &str,
        expected_version: u64,
        idempotency_key: &str,
    ) -> Result<ApprovalView> {
        let url = format!("{}/approvals/{request_id}/cancel", self.base_url);
        parse_response(
            self.client
                .post(url)
                .json(&serde_json::json!({
                    "expected_version": expected_version,
                    "idempotency_key": idempotency_key,
                }))
                .send()
                .await
                .map_err(|_| anyhow::anyhow!(APPROVAL_QUEUE_UNAVAILABLE))?,
        )
        .await
    }

    pub async fn chat_with_target(
        &self,
        message: String,
        channel: Option<&str>,
        chat_id: Option<&str>,
        event_tx: mpsc::UnboundedSender<AgentEvent>,
    ) -> Result<()> {
        let url = format!("{}/chat", self.base_url);
        let mut payload = serde_json::json!({ "message": message });
        if let Some(channel) = channel {
            payload["channel"] = serde_json::Value::String(channel.to_string());
        }
        if let Some(chat_id) = chat_id {
            payload["chat_id"] = serde_json::Value::String(chat_id.to_string());
        }
        let response = self.client.post(&url).json(&payload).send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Server returned error: {}", response.status());
        }

        let mut stream = response.bytes_stream().eventsource();

        while let Some(event) = stream.next().await {
            match event {
                Ok(event) => match event.event.as_str() {
                    "delta" => {
                        let _ = event_tx.send(AgentEvent::AssistantDelta { text: event.data });
                    }
                    "final" => {
                        let _ = event_tx.send(AgentEvent::FinalResponse {
                            content: event.data,
                        });
                    }
                    "tool_start" => {
                        if let Ok(data) = serde_json::from_str::<ToolStartEvent>(&event.data) {
                            let _ = event_tx.send(AgentEvent::ToolCallStarted {
                                name: data.name,
                                args_preview: data.args_preview,
                                call_id: data.id,
                            });
                        }
                    }
                    "tool_finish" => {
                        if let Ok(data) = serde_json::from_str::<ToolFinishEvent>(&event.data) {
                            let _ = event_tx.send(AgentEvent::ToolCallFinished {
                                name: data.name,
                                result: data.result,
                                is_error: data.error,
                                call_id: data.id,
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
                        let _ = event_tx.send(AgentEvent::Error {
                            message: event.data,
                        });
                    }
                    "turn_plan_updated" => {
                        if let Ok(args) = serde_json::from_str::<UpdatePlanArgs>(&event.data) {
                            let _ = event_tx.send(AgentEvent::ChatPlanUpdate { args });
                        }
                    }
                    "context_compaction" => {
                        if let Ok(data) = serde_json::from_str::<Value>(&event.data) {
                            let session_id = data
                                .get("session_id")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string();
                            let trigger = data
                                .get("trigger")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string();
                            let phase = data
                                .get("phase")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string();
                            let summary = data
                                .get("summary")
                                .and_then(Value::as_str)
                                .map(str::to_string);
                            let _ = event_tx.send(AgentEvent::ContextCompaction {
                                session_id,
                                trigger,
                                phase,
                                summary,
                            });
                        }
                    }
                    _ => {}
                },
                Err(e) => {
                    let _ = event_tx.send(AgentEvent::Error {
                        message: e.to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    pub async fn stop(&self, channel: Option<&str>, chat_id: Option<&str>) -> Result<bool> {
        let url = format!("{}/chat/stop", self.base_url);
        let mut payload = serde_json::json!({});
        if let Some(channel) = channel {
            payload["channel"] = serde_json::Value::String(channel.to_string());
        }
        if let Some(chat_id) = chat_id {
            payload["chat_id"] = serde_json::Value::String(chat_id.to_string());
        }

        let response = self.client.post(&url).json(&payload).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("Server returned error: {}", response.status());
        }

        let body: Value = response.json().await?;
        if body.get("status").and_then(|v| v.as_str()) != Some("ok") {
            let msg = body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            anyhow::bail!("Stop failed: {}", msg);
        }
        Ok(body
            .get("stopped")
            .and_then(|v| v.as_bool())
            .unwrap_or(true))
    }
}

async fn parse_response<T: serde::de::DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    let status = response.status();
    let bytes = response.bytes().await?;
    if !status.is_success() {
        if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
            let reason = value
                .get("reason_code")
                .and_then(Value::as_str)
                .unwrap_or("approval_request_failed");
            anyhow::bail!("{reason}");
        }
        anyhow::bail!("Server returned error: {status}");
    }
    Ok(serde_json::from_slice(&bytes)?)
}
