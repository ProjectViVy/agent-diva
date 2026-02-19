//! Cron tool for scheduling reminders and tasks

use crate::base::{Tool, ToolError};
use async_trait::async_trait;
use agent_diva_core::cron::{CronSchedule, CronService};
use serde_json::{json, Value};
use std::sync::Arc;

/// Cron tool for scheduling reminders and recurring tasks
pub struct CronTool {
    cron_service: Arc<CronService>,
    channel: Arc<tokio::sync::RwLock<String>>,
    chat_id: Arc<tokio::sync::RwLock<String>>,
}

impl CronTool {
    /// Create a new cron tool
    pub fn new(cron_service: Arc<CronService>) -> Self {
        Self {
            cron_service,
            channel: Arc::new(tokio::sync::RwLock::new(String::new())),
            chat_id: Arc::new(tokio::sync::RwLock::new(String::new())),
        }
    }

    /// Set the current session context for delivery
    pub async fn set_context(&self, channel: String, chat_id: String) {
        *self.channel.write().await = channel;
        *self.chat_id.write().await = chat_id;
    }

    /// Add a job
    async fn add_job(
        &self,
        message: String,
        every_seconds: Option<i64>,
        cron_expr: Option<String>,
        at: Option<String>,
        timezone: Option<String>,
    ) -> String {
        if message.is_empty() {
            return "Error: message is required for add".to_string();
        }

        let channel = self.channel.read().await.clone();
        let chat_id = self.chat_id.read().await.clone();

        if channel.is_empty() || chat_id.is_empty() {
            return "Error: no session context (channel/chat_id)".to_string();
        }

        // Build schedule
        let schedule = if let Some(seconds) = every_seconds {
            CronSchedule::every(seconds * 1000)
        } else if let Some(expr) = cron_expr {
            CronSchedule::cron(expr, timezone)
        } else if let Some(iso_time) = at {
            match chrono::DateTime::parse_from_rfc3339(&iso_time) {
                Ok(dt) => CronSchedule::at(dt.timestamp_millis()),
                Err(e) => return format!("Error: invalid ISO time '{}': {}", iso_time, e),
            }
        } else {
            return "Error: either every_seconds, cron_expr, or at is required".to_string();
        };

        let name = if message.len() > 30 {
            let mut end = 30;
            while !message.is_char_boundary(end) {
                end -= 1;
            }
            message[..end].to_string()
        } else {
            message.clone()
        };

        let job = self
            .cron_service
            .add_job(
                name,
                schedule,
                message,
                true,
                Some(channel),
                Some(chat_id),
                false,
            )
            .await;

        format!("Created job '{}' (id: {})", job.name, job.id)
    }

    /// List all jobs
    async fn list_jobs(&self) -> String {
        let jobs = self.cron_service.list_jobs(false).await;
        if jobs.is_empty() {
            return "No scheduled jobs.".to_string();
        }

        let lines: Vec<String> = jobs
            .iter()
            .map(|j| {
                let kind = match &j.schedule {
                    CronSchedule::At { .. } => "at",
                    CronSchedule::Every { .. } => "every",
                    CronSchedule::Cron { .. } => "cron",
                };
                format!("- {} (id: {}, {})", j.name, j.id, kind)
            })
            .collect();

        format!("Scheduled jobs:\n{}", lines.join("\n"))
    }

    /// Remove a job
    async fn remove_job(&self, job_id: Option<String>) -> String {
        match job_id {
            Some(id) => {
                if self.cron_service.remove_job(&id).await {
                    format!("Removed job {}", id)
                } else {
                    format!("Job {} not found", id)
                }
            }
            None => "Error: job_id is required for remove".to_string(),
        }
    }
}

#[async_trait]
impl Tool for CronTool {
    fn name(&self) -> &str {
        "cron"
    }

    fn description(&self) -> &str {
        "Schedule reminders and recurring tasks. Actions: add, list, remove."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["add", "list", "remove"],
                    "description": "Action to perform"
                },
                "message": {
                    "type": "string",
                    "description": "Reminder message (for add)"
                },
                "every_seconds": {
                    "type": "integer",
                    "description": "Interval in seconds (for recurring tasks)"
                },
                "cron_expr": {
                    "type": "string",
                    "description": "Cron expression like '0 9 * * *' (for scheduled tasks)"
                },
                "at": {
                    "type": "string",
                    "description": "ISO 8601 timestamp (e.g. '2023-10-27T10:00:00Z') for one-time task"
                },
                "timezone": {
                    "type": "string",
                    "description": "Timezone for cron expression (e.g. 'America/New_York')"
                },
                "job_id": {
                    "type": "string",
                    "description": "Job ID (for remove)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: Value) -> std::result::Result<String, ToolError> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("'action' is required".to_string()))?;

        match action {
            "add" => {
                let message = args
                    .get("message")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default();
                let every_seconds = args.get("every_seconds").and_then(|v| v.as_i64());
                let cron_expr = args
                    .get("cron_expr")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let at = args
                    .get("at")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let timezone = args
                    .get("timezone")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Ok(self.add_job(message, every_seconds, cron_expr, at, timezone).await)
            }
            "list" => Ok(self.list_jobs().await),
            "remove" => {
                let job_id = args
                    .get("job_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Ok(self.remove_job(job_id).await)
            }
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown action: {}",
                action
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::cron::CronService;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_cron_tool_name() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("cron.json");
        let service = Arc::new(CronService::new(store_path, None));
        let tool = CronTool::new(service);
        assert_eq!(tool.name(), "cron");
    }

    #[tokio::test]
    async fn test_cron_tool_parameters() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("cron.json");
        let service = Arc::new(CronService::new(store_path, None));
        let tool = CronTool::new(service);
        let params = tool.parameters();
        assert!(params["properties"]["action"].is_object());
        assert_eq!(params["required"][0], "action");
    }

    #[tokio::test]
    async fn test_cron_tool_add_job() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("cron.json");
        let service = Arc::new(CronService::new(store_path, None));
        service.start().await;

        let tool = CronTool::new(Arc::clone(&service));
        tool.set_context("test".to_string(), "123".to_string())
            .await;

        let args = json!({
            "action": "add",
            "message": "Test reminder",
            "every_seconds": 60
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("Created job"));

        // Verify job was created
        let jobs = service.list_jobs(false).await;
        assert_eq!(jobs.len(), 1);

        service.stop().await;
    }

    #[tokio::test]
    async fn test_cron_tool_list_jobs() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("cron.json");
        let service = Arc::new(CronService::new(store_path, None));
        service.start().await;

        let tool = CronTool::new(Arc::clone(&service));
        tool.set_context("test".to_string(), "123".to_string())
            .await;

        // Add a job first
        tool.execute(json!({
            "action": "add",
            "message": "Test",
            "every_seconds": 60
        }))
        .await
        .unwrap();

        // List jobs
        let result = tool.execute(json!({ "action": "list" })).await.unwrap();
        assert!(result.contains("Scheduled jobs"));
        assert!(result.contains("Test"));

        service.stop().await;
    }

    #[tokio::test]
    async fn test_cron_tool_remove_job() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("cron.json");
        let service = Arc::new(CronService::new(store_path, None));
        service.start().await;

        let tool = CronTool::new(Arc::clone(&service));
        tool.set_context("test".to_string(), "123".to_string())
            .await;

        // Add a job
        let add_result = tool
            .execute(json!({
                "action": "add",
                "message": "Test",
                "every_seconds": 60
            }))
            .await
            .unwrap();

        // Extract job ID from result
        let job_id = add_result
            .split("id: ")
            .nth(1)
            .and_then(|s| s.split(')').nth(0))
            .unwrap();

        // Remove the job
        let result = tool
            .execute(json!({
                "action": "remove",
                "job_id": job_id
            }))
            .await
            .unwrap();
        assert!(result.contains("Removed job"));

        service.stop().await;
    }

    #[tokio::test]
    async fn test_cron_tool_missing_context() {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("cron.json");
        let service = Arc::new(CronService::new(store_path, None));
        service.start().await;

        let tool = CronTool::new(service);

        let result = tool
            .execute(json!({
                "action": "add",
                "message": "Test",
                "every_seconds": 60
            }))
            .await
            .unwrap();

        assert!(result.contains("no session context"));
    }
}
