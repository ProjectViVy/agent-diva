//! Heartbeat service for periodic agent wake-up

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

use crate::heartbeat::types::{
    is_heartbeat_empty, HeartbeatConfig, HEARTBEAT_OK_TOKEN, HEARTBEAT_PROMPT,
};

/// Callback function type for heartbeat execution
pub type HeartbeatCallback = Arc<
    dyn Fn(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send>>
        + Send
        + Sync,
>;

/// Periodic heartbeat service that wakes the agent to check for tasks
///
/// The agent reads HEARTBEAT.md from the workspace and executes any
/// tasks listed there. If nothing needs attention, it replies HEARTBEAT_OK.
pub struct HeartbeatService {
    workspace: PathBuf,
    config: HeartbeatConfig,
    on_heartbeat: Option<HeartbeatCallback>,
    running: Arc<RwLock<bool>>,
    task: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl HeartbeatService {
    /// Create a new heartbeat service
    pub fn new(
        workspace: PathBuf,
        config: HeartbeatConfig,
        on_heartbeat: Option<HeartbeatCallback>,
    ) -> Self {
        Self {
            workspace,
            config,
            on_heartbeat,
            running: Arc::new(RwLock::new(false)),
            task: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the path to HEARTBEAT.md
    fn heartbeat_file(&self) -> PathBuf {
        self.workspace.join("HEARTBEAT.md")
    }

    /// Read HEARTBEAT.md content
    #[allow(dead_code)]
    async fn read_heartbeat_file(&self) -> Option<String> {
        let path = self.heartbeat_file();
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

    /// Start the heartbeat service
    pub async fn start(&self) {
        if !self.config.enabled {
            info!("Heartbeat disabled");
            return;
        }

        // Check if already running
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
        let on_heartbeat = self.on_heartbeat.clone();
        let workspace = self.workspace.clone();

        let task = tokio::spawn(async move {
            let service = HeartbeatServiceHandle {
                workspace,
                on_heartbeat,
                running: Arc::clone(&running),
            };
            service.run_loop(interval_s).await;
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

    /// Manually trigger a heartbeat
    pub async fn trigger_now(&self) -> Option<String> {
        if let Some(callback) = &self.on_heartbeat {
            Some((callback)(HEARTBEAT_PROMPT.to_string()).await)
        } else {
            None
        }
    }

    /// Get service status
    pub async fn status(&self) -> serde_json::Value {
        let is_running = *self.running.read().await;
        let has_callback = self.on_heartbeat.is_some();
        let heartbeat_file_exists = self.heartbeat_file().exists();

        serde_json::json!({
            "enabled": self.config.enabled,
            "running": is_running,
            "interval_s": self.config.interval_s,
            "has_callback": has_callback,
            "heartbeat_file_exists": heartbeat_file_exists,
        })
    }
}

/// Handle for async task (to avoid circular references)
struct HeartbeatServiceHandle {
    workspace: PathBuf,
    on_heartbeat: Option<HeartbeatCallback>,
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
        let content = self.read_heartbeat_file().await;

        // Skip if HEARTBEAT.md is empty or doesn't exist
        if is_heartbeat_empty(content.as_deref()) {
            debug!("Heartbeat: no tasks (HEARTBEAT.md empty)");
            return Ok(());
        }

        info!("Heartbeat: checking for tasks...");

        if let Some(callback) = &self.on_heartbeat {
            let response = (callback)(HEARTBEAT_PROMPT.to_string()).await;

            // Check if agent said "nothing to do" (case-insensitive, ignore underscores)
            let normalized_response: String = response
                .to_uppercase()
                .chars()
                .filter(|&c| c != '_')
                .collect();
            let normalized_token: String = HEARTBEAT_OK_TOKEN
                .to_uppercase()
                .chars()
                .filter(|&c| c != '_')
                .collect();

            if normalized_response.contains(&normalized_token) {
                info!("Heartbeat: OK (no action needed)");
            } else {
                info!("Heartbeat: completed task");
            }
        }

        Ok(())
    }

    async fn read_heartbeat_file(&self) -> Option<String> {
        let path = self.workspace.join("HEARTBEAT.md");
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heartbeat::types::DEFAULT_HEARTBEAT_INTERVAL_S;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_heartbeat_service_new() {
        let temp_dir = TempDir::new().unwrap();
        let config = HeartbeatConfig::default();
        let service = HeartbeatService::new(temp_dir.path().to_path_buf(), config, None);

        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_heartbeat_service_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let config = HeartbeatConfig {
            enabled: false,
            interval_s: 60,
        };
        let service = HeartbeatService::new(temp_dir.path().to_path_buf(), config, None);

        service.start().await;
        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_heartbeat_service_start_stop() {
        let temp_dir = TempDir::new().unwrap();
        let config = HeartbeatConfig {
            enabled: true,
            interval_s: 3600, // Long interval so it doesn't tick during test
        };
        let service = HeartbeatService::new(temp_dir.path().to_path_buf(), config, None);

        service.start().await;
        assert!(service.is_running().await);

        service.stop().await;
        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_heartbeat_service_status() {
        let temp_dir = TempDir::new().unwrap();
        let config = HeartbeatConfig::default();
        let service = HeartbeatService::new(temp_dir.path().to_path_buf(), config, None);

        let status = service.status().await;
        assert!(status["enabled"].as_bool().unwrap());
        assert!(!status["running"].as_bool().unwrap());
        assert_eq!(
            status["interval_s"].as_i64().unwrap(),
            DEFAULT_HEARTBEAT_INTERVAL_S
        );
    }

    #[tokio::test]
    async fn test_heartbeat_service_trigger_now_no_callback() {
        let temp_dir = TempDir::new().unwrap();
        let config = HeartbeatConfig::default();
        let service = HeartbeatService::new(temp_dir.path().to_path_buf(), config, None);

        let result = service.trigger_now().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_heartbeat_service_trigger_now_with_callback() {
        let temp_dir = TempDir::new().unwrap();
        let config = HeartbeatConfig::default();

        let callback: HeartbeatCallback = Arc::new(|prompt: String| {
            Box::pin(async move {
                assert!(prompt.contains("HEARTBEAT.md"));
                "HEARTBEAT_OK".to_string()
            })
        });

        let service = HeartbeatService::new(temp_dir.path().to_path_buf(), config, Some(callback));

        let result = service.trigger_now().await;
        assert_eq!(result.unwrap(), "HEARTBEAT_OK");
    }

    #[tokio::test]
    async fn test_heartbeat_service_with_heartbeat_file() {
        let temp_dir = TempDir::new().unwrap();
        let heartbeat_file = temp_dir.path().join("HEARTBEAT.md");

        // Create HEARTBEAT.md with actionable content
        tokio::fs::write(&heartbeat_file, "# Tasks\n\nCheck the logs")
            .await
            .unwrap();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let callback: HeartbeatCallback = Arc::new(move |_prompt: String| {
            let counter = Arc::clone(&counter_clone);
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                "HEARTBEAT_OK".to_string()
            })
        });

        let config = HeartbeatConfig {
            enabled: true,
            interval_s: 1, // Short interval for testing
        };

        let service = HeartbeatService::new(temp_dir.path().to_path_buf(), config, Some(callback));

        service.start().await;

        // Wait for at least one tick
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

        service.stop().await;

        // Should have ticked at least once
        assert!(counter.load(Ordering::SeqCst) >= 1);
    }

    #[tokio::test]
    async fn test_heartbeat_service_empty_file_no_tick() {
        let temp_dir = TempDir::new().unwrap();
        let heartbeat_file = temp_dir.path().join("HEARTBEAT.md");

        // Create empty HEARTBEAT.md
        tokio::fs::write(&heartbeat_file, "# Title\n\n<!-- comment -->\n- [ ]\n")
            .await
            .unwrap();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let callback: HeartbeatCallback = Arc::new(move |_prompt: String| {
            let counter = Arc::clone(&counter_clone);
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                "HEARTBEAT_OK".to_string()
            })
        });

        let config = HeartbeatConfig {
            enabled: true,
            interval_s: 1,
        };

        let service = HeartbeatService::new(temp_dir.path().to_path_buf(), config, Some(callback));

        service.start().await;

        // Wait for potential tick
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

        service.stop().await;

        // Should not have ticked (file is empty)
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }
}
