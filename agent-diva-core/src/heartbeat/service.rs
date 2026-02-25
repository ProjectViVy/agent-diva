//! Heartbeat service for periodic agent wake-up

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::future::Future;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::heartbeat::types::{is_heartbeat_empty, HeartbeatConfig, HeartbeatDecision};

/// Callback for the LLM decision phase: takes HEARTBEAT.md content and returns a HeartbeatDecision.
pub type HeartbeatDecideCallback = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<HeartbeatDecision, Box<dyn std::error::Error + Send + Sync>>> + Send>>
        + Send
        + Sync,
>;

/// Callback for the task execution phase: takes a tasks summary string and runs the agent loop.
pub type HeartbeatExecuteCallback = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = String> + Send>>
        + Send
        + Sync,
>;

/// Periodic heartbeat service that wakes the agent to check for tasks.
///
/// Two-phase design:
/// 1. **Decide** — read HEARTBEAT.md, call the LLM with a tool to decide skip/run.
/// 2. **Execute** — if the decision is "run", invoke the full agent loop with the tasks summary.
pub struct HeartbeatService {
    workspace: PathBuf,
    config: HeartbeatConfig,
    on_decide: Option<HeartbeatDecideCallback>,
    on_execute: Option<HeartbeatExecuteCallback>,
    running: Arc<RwLock<bool>>,
    task: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl HeartbeatService {
    /// Create a new heartbeat service with decide + execute callbacks.
    pub fn new(
        workspace: PathBuf,
        config: HeartbeatConfig,
        on_decide: Option<HeartbeatDecideCallback>,
        on_execute: Option<HeartbeatExecuteCallback>,
    ) -> Self {
        Self {
            workspace,
            config,
            on_decide,
            on_execute,
            running: Arc::new(RwLock::new(false)),
            task: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the path to HEARTBEAT.md
    fn heartbeat_file(&self) -> PathBuf {
        self.workspace.join("HEARTBEAT.md")
    }

    /// Start the heartbeat service
    pub async fn start(&self) {
        if !self.config.enabled {
            info!("Heartbeat disabled");
            return;
        }

        {
            let running_guard = self.running.read().await;
            if *running_guard {
                debug!("Heartbeat service already running");
                return;
            }
        }

        *self.running.write().await = true;

        let interval_s = self.config.interval_s;
        let running = Arc::clone(&self.running);
        let on_decide = self.on_decide.clone();
        let on_execute = self.on_execute.clone();
        let workspace = self.workspace.clone();

        let task = tokio::spawn(async move {
            let handle = HeartbeatServiceHandle {
                workspace,
                on_decide,
                on_execute,
                running: Arc::clone(&running),
            };
            handle.run_loop(interval_s).await;
        });

        *self.task.write().await = Some(task);
        info!("Heartbeat started (every {}s)", interval_s);
    }

    /// Stop the heartbeat service
    pub async fn stop(&self) {
        *self.running.write().await = false;

        let mut task_guard = self.task.write().await;
        if let Some(task) = task_guard.take() {
            task.abort();
        }
    }

    /// Check if the service is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Manually trigger a heartbeat tick (decide + optionally execute).
    pub async fn trigger_now(&self) -> Option<String> {
        let on_decide = self.on_decide.as_ref()?;
        let workspace = self.workspace.clone();

        let content = read_heartbeat_file(&workspace).await;
        if is_heartbeat_empty(content.as_deref()) {
            return Some("skip (empty)".to_string());
        }

        let heartbeat_content = content.unwrap_or_default();
        match (on_decide)(heartbeat_content).await {
            Ok(decision) if decision.is_run() => {
                let tasks = decision.tasks.unwrap_or_default();
                if let Some(on_execute) = &self.on_execute {
                    let result = (on_execute)(tasks).await;
                    Some(result)
                } else {
                    Some("run (no execute callback)".to_string())
                }
            }
            Ok(_) => Some("skip".to_string()),
            Err(e) => {
                warn!("Heartbeat decide error: {}", e);
                Some(format!("error: {}", e))
            }
        }
    }

    /// Get service status
    pub async fn status(&self) -> serde_json::Value {
        let is_running = *self.running.read().await;
        let has_decide = self.on_decide.is_some();
        let has_execute = self.on_execute.is_some();
        let heartbeat_file_exists = self.heartbeat_file().exists();

        serde_json::json!({
            "enabled": self.config.enabled,
            "running": is_running,
            "interval_s": self.config.interval_s,
            "has_decide_callback": has_decide,
            "has_execute_callback": has_execute,
            "heartbeat_file_exists": heartbeat_file_exists,
        })
    }
}

/// Shared helper to read HEARTBEAT.md
async fn read_heartbeat_file(workspace: &Path) -> Option<String> {
    let path = workspace.join("HEARTBEAT.md");
    if path.exists() {
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => Some(content),
            Err(e) => {
                debug!("Failed to read HEARTBEAT.md: {}", e);
                None
            }
        }
    } else {
        None
    }
}

/// Handle for async task (to avoid circular references)
struct HeartbeatServiceHandle {
    workspace: PathBuf,
    on_decide: Option<HeartbeatDecideCallback>,
    on_execute: Option<HeartbeatExecuteCallback>,
    running: Arc<RwLock<bool>>,
}

impl HeartbeatServiceHandle {
    async fn run_loop(&self, interval_s: i64) {
        let interval = tokio::time::Duration::from_secs(interval_s as u64);

        loop {
            tokio::time::sleep(interval).await;

            let is_running = *self.running.read().await;
            if !is_running {
                break;
            }

            if let Err(e) = self.tick().await {
                error!("Heartbeat error: {}", e);
            }
        }
    }

    async fn tick(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let content = read_heartbeat_file(&self.workspace).await;

        // Skip if HEARTBEAT.md is empty or doesn't exist
        if is_heartbeat_empty(content.as_deref()) {
            debug!("Heartbeat: no tasks (HEARTBEAT.md empty)");
            return Ok(());
        }

        info!("Heartbeat: checking for tasks...");

        let on_decide = match &self.on_decide {
            Some(cb) => cb,
            None => {
                debug!("Heartbeat: no decide callback");
                return Ok(());
            }
        };

        let heartbeat_content = content.unwrap_or_default();
        let decision = match (on_decide)(heartbeat_content).await {
            Ok(d) => d,
            Err(e) => {
                warn!("Heartbeat decide failed, defaulting to skip: {}", e);
                HeartbeatDecision { action: "skip".to_string(), tasks: None }
            }
        };

        if decision.is_run() {
            let tasks = decision.tasks.unwrap_or_default();
            info!("Heartbeat: running tasks");
            if let Some(on_execute) = &self.on_execute {
                let _result = (on_execute)(tasks).await;
                info!("Heartbeat: completed task execution");
            } else {
                warn!("Heartbeat: decision was 'run' but no execute callback");
            }
        } else {
            info!("Heartbeat: OK (no action needed)");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heartbeat::types::DEFAULT_HEARTBEAT_INTERVAL_S;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    /// Helper: build a decide callback that always returns "skip".
    fn skip_decide() -> HeartbeatDecideCallback {
        Arc::new(|_content: String| {
            Box::pin(async move {
                Ok(HeartbeatDecision { action: "skip".to_string(), tasks: None })
            })
        })
    }

    /// Helper: build a decide callback that always returns "run" with given tasks.
    fn run_decide(tasks: &str) -> HeartbeatDecideCallback {
        let tasks = tasks.to_string();
        Arc::new(move |_content: String| {
            let tasks = tasks.clone();
            Box::pin(async move {
                Ok(HeartbeatDecision { action: "run".to_string(), tasks: Some(tasks) })
            })
        })
    }

    #[tokio::test]
    async fn test_heartbeat_service_new() {
        let temp_dir = TempDir::new().unwrap();
        let config = HeartbeatConfig::default();
        let service = HeartbeatService::new(temp_dir.path().to_path_buf(), config, None, None);
        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_heartbeat_service_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let config = HeartbeatConfig { enabled: false, interval_s: 60 };
        let service = HeartbeatService::new(temp_dir.path().to_path_buf(), config, None, None);
        service.start().await;
        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_heartbeat_service_start_stop() {
        let temp_dir = TempDir::new().unwrap();
        let config = HeartbeatConfig { enabled: true, interval_s: 3600 };
        let service = HeartbeatService::new(temp_dir.path().to_path_buf(), config, None, None);
        service.start().await;
        assert!(service.is_running().await);
        service.stop().await;
        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_heartbeat_service_status() {
        let temp_dir = TempDir::new().unwrap();
        let config = HeartbeatConfig::default();
        let service = HeartbeatService::new(temp_dir.path().to_path_buf(), config, None, None);
        let status = service.status().await;
        assert!(status["enabled"].as_bool().unwrap());
        assert!(!status["running"].as_bool().unwrap());
        assert_eq!(status["interval_s"].as_i64().unwrap(), DEFAULT_HEARTBEAT_INTERVAL_S);
    }

    #[tokio::test]
    async fn test_heartbeat_trigger_now_no_callback() {
        let temp_dir = TempDir::new().unwrap();
        let config = HeartbeatConfig::default();
        let service = HeartbeatService::new(temp_dir.path().to_path_buf(), config, None, None);
        let result = service.trigger_now().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_heartbeat_trigger_now_skip() {
        let temp_dir = TempDir::new().unwrap();
        // Write actionable content so it doesn't short-circuit
        tokio::fs::write(temp_dir.path().join("HEARTBEAT.md"), "Do something")
            .await.unwrap();
        let config = HeartbeatConfig::default();
        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(), config, Some(skip_decide()), None,
        );
        let result = service.trigger_now().await.unwrap();
        assert_eq!(result, "skip");
    }

    #[tokio::test]
    async fn test_heartbeat_trigger_now_run_calls_execute() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("HEARTBEAT.md"), "Check logs")
            .await.unwrap();

        let execute_counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&execute_counter);
        let on_execute: HeartbeatExecuteCallback = Arc::new(move |tasks: String| {
            let counter = Arc::clone(&counter_clone);
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                format!("executed: {}", tasks)
            })
        });

        let config = HeartbeatConfig::default();
        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(), config,
            Some(run_decide("Check logs")), Some(on_execute),
        );
        let result = service.trigger_now().await.unwrap();
        assert!(result.contains("executed"));
        assert_eq!(execute_counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_heartbeat_skip_no_execute_called() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("HEARTBEAT.md"), "# Tasks\n\nCheck logs")
            .await.unwrap();

        let decide_counter = Arc::new(AtomicUsize::new(0));
        let dc = Arc::clone(&decide_counter);
        let on_decide: HeartbeatDecideCallback = Arc::new(move |_content: String| {
            let dc = Arc::clone(&dc);
            Box::pin(async move {
                dc.fetch_add(1, Ordering::SeqCst);
                Ok(HeartbeatDecision { action: "skip".to_string(), tasks: None })
            })
        });

        let execute_counter = Arc::new(AtomicUsize::new(0));
        let ec = Arc::clone(&execute_counter);
        let on_execute: HeartbeatExecuteCallback = Arc::new(move |_tasks: String| {
            let ec = Arc::clone(&ec);
            Box::pin(async move {
                ec.fetch_add(1, Ordering::SeqCst);
                "done".to_string()
            })
        });

        let config = HeartbeatConfig { enabled: true, interval_s: 1 };
        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(), config,
            Some(on_decide), Some(on_execute),
        );
        service.start().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
        service.stop().await;

        assert!(decide_counter.load(Ordering::SeqCst) >= 1);
        assert_eq!(execute_counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_heartbeat_empty_file_no_decide() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(
            temp_dir.path().join("HEARTBEAT.md"),
            "# Title\n\n<!-- comment -->\n- [ ]\n",
        ).await.unwrap();

        let decide_counter = Arc::new(AtomicUsize::new(0));
        let dc = Arc::clone(&decide_counter);
        let on_decide: HeartbeatDecideCallback = Arc::new(move |_content: String| {
            let dc = Arc::clone(&dc);
            Box::pin(async move {
                dc.fetch_add(1, Ordering::SeqCst);
                Ok(HeartbeatDecision { action: "skip".to_string(), tasks: None })
            })
        });

        let config = HeartbeatConfig { enabled: true, interval_s: 1 };
        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(), config, Some(on_decide), None,
        );
        service.start().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
        service.stop().await;

        // Decide should NOT have been called (file is empty/non-actionable)
        assert_eq!(decide_counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_heartbeat_malformed_decide_defaults_to_skip() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("HEARTBEAT.md"), "Do something")
            .await.unwrap();

        let on_decide: HeartbeatDecideCallback = Arc::new(|_content: String| {
            Box::pin(async move {
                Err("LLM error".into())
            })
        });

        let execute_counter = Arc::new(AtomicUsize::new(0));
        let ec = Arc::clone(&execute_counter);
        let on_execute: HeartbeatExecuteCallback = Arc::new(move |_tasks: String| {
            let ec = Arc::clone(&ec);
            Box::pin(async move {
                ec.fetch_add(1, Ordering::SeqCst);
                "done".to_string()
            })
        });

        let config = HeartbeatConfig { enabled: true, interval_s: 1 };
        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(), config,
            Some(on_decide), Some(on_execute),
        );
        service.start().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
        service.stop().await;

        // Execute should NOT have been called (error defaults to skip)
        assert_eq!(execute_counter.load(Ordering::SeqCst), 0);
    }
}
