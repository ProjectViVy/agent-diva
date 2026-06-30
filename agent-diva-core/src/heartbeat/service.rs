//! Heartbeat service for periodic agent wake-up

use std::error::Error as StdError;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::bus::{AgentBusEvent, MessageBus};
use crate::heartbeat::types::{is_heartbeat_empty, HeartbeatConfig, HeartbeatDecision};
use crate::presence::{HeartbeatRhythm, PresenceManager};

type HeartbeatError = Box<dyn StdError + Send + Sync>;

/// Callback for the LLM decision phase: takes HEARTBEAT.md content and returns a HeartbeatDecision.
pub type HeartbeatDecideCallback = Arc<
    dyn Fn(
            String,
        )
            -> Pin<Box<dyn Future<Output = Result<HeartbeatDecision, HeartbeatError>> + Send>>
        + Send
        + Sync,
>;

/// Callback for the task execution phase: takes a tasks summary string and runs the agent loop.
pub type HeartbeatExecuteCallback =
    Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = String> + Send>> + Send + Sync>;

/// Periodic heartbeat service that wakes the agent to check for tasks.
///
/// Two-phase design:
/// 1. **Decide**: read HEARTBEAT.md, call the LLM with a tool to decide skip/run.
/// 2. **Execute**: if the decision is "run", invoke the full agent loop with the tasks summary.
///
/// The heartbeat cadence adapts to user presence via [`HeartbeatRhythm`]:
/// - **Normal**: user is active, heartbeat at base interval.
/// - **Slow**: user is distracted/gone, heartbeat at reduced frequency.
/// - **Suspended**: user is away, heartbeat paused.
pub struct HeartbeatService {
    workspace: PathBuf,
    config: Arc<std::sync::RwLock<HeartbeatConfig>>,
    bus: Option<MessageBus>,
    presence: PresenceManager,
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
        bus: Option<MessageBus>,
        presence: PresenceManager,
        on_decide: Option<HeartbeatDecideCallback>,
        on_execute: Option<HeartbeatExecuteCallback>,
    ) -> Self {
        Self {
            workspace,
            config: Arc::new(std::sync::RwLock::new(config)),
            bus,
            presence,
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
        if !self
            .config
            .read()
            .expect("heartbeat config lock poisoned")
            .enabled
        {
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

        let running = Arc::clone(&self.running);
        let bus = self.bus.clone();
        let config = Arc::clone(&self.config);
        let presence = self.presence.clone();
        let on_decide = self.on_decide.clone();
        let on_execute = self.on_execute.clone();
        let workspace = self.workspace.clone();

        let task = tokio::spawn(async move {
            let handle = HeartbeatServiceHandle {
                workspace,
                config,
                bus,
                presence,
                on_decide,
                on_execute,
                running: Arc::clone(&running),
            };
            handle.run_loop().await;
        });

        *self.task.write().await = Some(task);
        let config = self
            .config
            .read()
            .expect("heartbeat config lock poisoned")
            .clone();
        info!(
            "Heartbeat started (every {}s, decide retries {})",
            config.interval_s, config.decide_max_retries
        );
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
        if let Some(bus) = &self.bus {
            bus.refresh_presence();
        }

        let content = read_heartbeat_file(&self.workspace).await;
        if is_heartbeat_empty(content.as_deref()) {
            emit_heartbeat_event(self.bus.as_ref(), "skip", "");
            return Some("skip (empty)".to_string());
        }

        let heartbeat_content = content.unwrap_or_default();
        match retry_decide_call(&self.config, on_decide, heartbeat_content).await {
            Ok(decision) if decision.is_run() => {
                let tasks = decision.tasks.unwrap_or_default();
                emit_heartbeat_event(self.bus.as_ref(), "run", &tasks);
                if let Some(on_execute) = &self.on_execute {
                    Some((on_execute)(tasks).await)
                } else {
                    Some("run (no execute callback)".to_string())
                }
            }
            Ok(_) => {
                emit_heartbeat_event(self.bus.as_ref(), "skip", "");
                Some("skip".to_string())
            }
            Err(error) => {
                error!("Heartbeat decide exhausted retries: {}", error);
                emit_heartbeat_event(self.bus.as_ref(), "error", "");
                Some(format!("error: {}", error))
            }
        }
    }

    /// Get service status
    pub async fn status(&self) -> serde_json::Value {
        let is_running = *self.running.read().await;
        let config = self
            .config
            .read()
            .expect("heartbeat config lock poisoned")
            .clone();
        let has_decide = self.on_decide.is_some();
        let has_execute = self.on_execute.is_some();
        let heartbeat_file_exists = self.heartbeat_file().exists();

        serde_json::json!({
            "enabled": config.enabled,
            "running": is_running,
            "interval_s": config.interval_s,
            "decide_max_retries": config.decide_max_retries,
            "decide_backoff_ms": config.decide_backoff_ms,
            "decide_max_backoff_ms": config.decide_max_backoff_ms,
            "has_decide_callback": has_decide,
            "has_execute_callback": has_execute,
            "heartbeat_file_exists": heartbeat_file_exists,
        })
    }

    pub fn update_config(&self, config: HeartbeatConfig) {
        *self.config.write().expect("heartbeat config lock poisoned") = config;
    }
}

fn emit_heartbeat_event(bus: Option<&MessageBus>, state: &str, tasks: &str) {
    if let Some(bus) = bus {
        let _ = bus.emit(AgentBusEvent::HeartbeatTriggered {
            state: state.to_string(),
            tasks: tasks.to_string(),
        });
    }
}

fn minimum_one_second(seconds: f64) -> Duration {
    Duration::from_secs(seconds.max(1.0).floor() as u64)
}

fn base_interval(config: &HeartbeatConfig) -> Duration {
    minimum_one_second(config.interval_s as f64)
}

fn effective_interval_for_rhythm(
    config: &HeartbeatConfig,
    rhythm: HeartbeatRhythm,
    distracted_multiplier: f64,
) -> Option<Duration> {
    match rhythm {
        HeartbeatRhythm::Normal => Some(base_interval(config)),
        HeartbeatRhythm::Slow => Some(minimum_one_second(
            config.interval_s as f64 * distracted_multiplier,
        )),
        HeartbeatRhythm::Suspended => None,
    }
}

fn effective_interval_for_presence(
    config: &HeartbeatConfig,
    presence: &PresenceManager,
) -> Option<Duration> {
    let rhythm = HeartbeatRhythm::for_presence(presence.state());
    let multiplier = presence.config().distracted_heartbeat_multiplier;
    effective_interval_for_rhythm(config, rhythm, multiplier)
}

async fn retry_decide_call(
    config_lock: &Arc<std::sync::RwLock<HeartbeatConfig>>,
    on_decide: &HeartbeatDecideCallback,
    heartbeat_content: String,
) -> Result<HeartbeatDecision, HeartbeatError> {
    let config = config_lock
        .read()
        .expect("heartbeat config lock poisoned")
        .clone();
    let max_attempts = config.decide_max_retries.saturating_add(1);
    let mut attempt = 1;
    let mut delay_ms = config.decide_backoff_ms;

    loop {
        match (on_decide)(heartbeat_content.clone()).await {
            Ok(decision) => return Ok(decision),
            Err(error) if attempt < max_attempts => {
                let next_delay_ms = delay_ms.min(config.decide_max_backoff_ms);
                warn!(
                    "Heartbeat decide failed (attempt {}/{}), retrying in {}ms: {}",
                    attempt, max_attempts, next_delay_ms, error
                );
                tokio::time::sleep(Duration::from_millis(next_delay_ms)).await;
                delay_ms = delay_ms.saturating_mul(2).min(config.decide_max_backoff_ms);
                attempt += 1;
            }
            Err(error) => return Err(error),
        }
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
    config: Arc<std::sync::RwLock<HeartbeatConfig>>,
    bus: Option<MessageBus>,
    presence: PresenceManager,
    on_decide: Option<HeartbeatDecideCallback>,
    on_execute: Option<HeartbeatExecuteCallback>,
    running: Arc<RwLock<bool>>,
}

impl HeartbeatServiceHandle {
    async fn run_loop(&self) {
        loop {
            let current_config = self
                .config
                .read()
                .expect("heartbeat config lock poisoned")
                .clone();
            let base_interval = base_interval(&current_config);

            match effective_interval_for_presence(&current_config, &self.presence) {
                Some(interval) => tokio::time::sleep(interval).await,
                None => {
                    debug!("Heartbeat suspended (user away)");
                    tokio::time::sleep(base_interval).await;
                }
            }

            let is_running = *self.running.read().await;
            if !is_running {
                break;
            }

            if let Err(e) = self.tick().await {
                error!("Heartbeat error: {}", e);
            }
        }
    }

    async fn tick(&self) -> Result<(), HeartbeatError> {
        if let Some(bus) = &self.bus {
            bus.refresh_presence();
        }
        let content = read_heartbeat_file(&self.workspace).await;

        if is_heartbeat_empty(content.as_deref()) {
            debug!("Heartbeat: no tasks (HEARTBEAT.md empty)");
            emit_heartbeat_event(self.bus.as_ref(), "skip", "");
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
        let decision = match retry_decide_call(&self.config, on_decide, heartbeat_content).await {
            Ok(decision) => decision,
            Err(error) => {
                error!("Heartbeat decide exhausted retries: {}", error);
                emit_heartbeat_event(self.bus.as_ref(), "error", "");
                return Ok(());
            }
        };

        if decision.is_run() {
            let tasks = decision.tasks.unwrap_or_default();
            emit_heartbeat_event(self.bus.as_ref(), "run", &tasks);
            info!("Heartbeat: running tasks");
            if let Some(on_execute) = &self.on_execute {
                let _result = (on_execute)(tasks).await;
                info!("Heartbeat: completed task execution");
            } else {
                warn!("Heartbeat: decision was 'run' but no execute callback");
            }
        } else {
            emit_heartbeat_event(self.bus.as_ref(), "skip", "");
            info!("Heartbeat: OK (no action needed)");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heartbeat::types::DEFAULT_HEARTBEAT_INTERVAL_S;
    use crate::presence::{PresenceConfig, PresenceState};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;

    fn config_with_interval(interval_s: i64) -> HeartbeatConfig {
        HeartbeatConfig {
            interval_s,
            ..HeartbeatConfig::default()
        }
    }

    fn config_with_retry(
        interval_s: i64,
        retries: u32,
        backoff_ms: u64,
        max_backoff_ms: u64,
    ) -> HeartbeatConfig {
        HeartbeatConfig {
            interval_s,
            decide_max_retries: retries,
            decide_backoff_ms: backoff_ms,
            decide_max_backoff_ms: max_backoff_ms,
            ..HeartbeatConfig::default()
        }
    }

    /// Helper: build a decide callback that always returns "skip".
    fn skip_decide() -> HeartbeatDecideCallback {
        Arc::new(|_content: String| {
            Box::pin(async move {
                Ok(HeartbeatDecision {
                    action: "skip".to_string(),
                    tasks: None,
                })
            })
        })
    }

    /// Helper: build a decide callback that always returns "run" with given tasks.
    fn run_decide(tasks: &str) -> HeartbeatDecideCallback {
        let tasks = tasks.to_string();
        Arc::new(move |_content: String| {
            let tasks = tasks.clone();
            Box::pin(async move {
                Ok(HeartbeatDecision {
                    action: "run".to_string(),
                    tasks: Some(tasks),
                })
            })
        })
    }

    fn build_handle(
        workspace: PathBuf,
        config: HeartbeatConfig,
        bus: Option<MessageBus>,
        presence: PresenceManager,
        on_decide: Option<HeartbeatDecideCallback>,
        on_execute: Option<HeartbeatExecuteCallback>,
    ) -> HeartbeatServiceHandle {
        HeartbeatServiceHandle {
            workspace,
            config: Arc::new(std::sync::RwLock::new(config)),
            bus,
            presence,
            on_decide,
            on_execute,
            running: Arc::new(RwLock::new(true)),
        }
    }

    #[tokio::test]
    async fn test_heartbeat_service_new() {
        let temp_dir = TempDir::new().unwrap();
        let config = HeartbeatConfig::default();
        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(),
            config,
            None,
            PresenceManager::with_defaults(),
            None,
            None,
        );
        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_heartbeat_service_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let config = HeartbeatConfig {
            enabled: false,
            ..config_with_interval(60)
        };
        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(),
            config,
            None,
            PresenceManager::with_defaults(),
            None,
            None,
        );
        service.start().await;
        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_heartbeat_service_start_stop() {
        let temp_dir = TempDir::new().unwrap();
        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(),
            config_with_interval(3600),
            None,
            PresenceManager::with_defaults(),
            None,
            None,
        );
        service.start().await;
        assert!(service.is_running().await);
        service.stop().await;
        assert!(!service.is_running().await);
    }

    #[tokio::test]
    async fn test_heartbeat_service_status() {
        let temp_dir = TempDir::new().unwrap();
        let config = HeartbeatConfig::default();
        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(),
            config,
            None,
            PresenceManager::with_defaults(),
            None,
            None,
        );
        let status = service.status().await;
        assert!(status["enabled"].as_bool().unwrap());
        assert!(!status["running"].as_bool().unwrap());
        assert_eq!(
            status["interval_s"].as_i64().unwrap(),
            DEFAULT_HEARTBEAT_INTERVAL_S
        );
        assert_eq!(status["decide_max_retries"].as_u64().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_heartbeat_service_update_config_changes_status() {
        let temp_dir = TempDir::new().unwrap();
        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(),
            HeartbeatConfig::default(),
            None,
            PresenceManager::with_defaults(),
            None,
            None,
        );

        service.update_config(HeartbeatConfig {
            enabled: false,
            ..config_with_retry(17, 4, 123, 456)
        });

        let status = service.status().await;
        assert!(!status["enabled"].as_bool().unwrap());
        assert_eq!(status["interval_s"].as_i64().unwrap(), 17);
        assert_eq!(status["decide_max_retries"].as_u64().unwrap(), 4);
        assert_eq!(status["decide_backoff_ms"].as_u64().unwrap(), 123);
        assert_eq!(status["decide_max_backoff_ms"].as_u64().unwrap(), 456);
    }

    #[tokio::test]
    async fn test_heartbeat_trigger_now_no_callback() {
        let temp_dir = TempDir::new().unwrap();
        let config = HeartbeatConfig::default();
        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(),
            config,
            None,
            PresenceManager::with_defaults(),
            None,
            None,
        );
        let result = service.trigger_now().await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_heartbeat_trigger_now_skip() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("HEARTBEAT.md"), "Do something")
            .await
            .unwrap();
        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(),
            HeartbeatConfig::default(),
            None,
            PresenceManager::with_defaults(),
            Some(skip_decide()),
            None,
        );
        let result = service.trigger_now().await.unwrap();
        assert_eq!(result, "skip");
    }

    #[tokio::test]
    async fn test_heartbeat_trigger_now_run_calls_execute() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("HEARTBEAT.md"), "Check logs")
            .await
            .unwrap();

        let execute_counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&execute_counter);
        let on_execute: HeartbeatExecuteCallback = Arc::new(move |tasks: String| {
            let counter = Arc::clone(&counter_clone);
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                format!("executed: {}", tasks)
            })
        });

        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(),
            HeartbeatConfig::default(),
            None,
            PresenceManager::with_defaults(),
            Some(run_decide("Check logs")),
            Some(on_execute),
        );
        let result = service.trigger_now().await.unwrap();
        assert!(result.contains("executed"));
        assert_eq!(execute_counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_heartbeat_trigger_now_retries_until_success() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("HEARTBEAT.md"), "Check logs")
            .await
            .unwrap();

        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_cb = Arc::clone(&attempts);
        let on_decide: HeartbeatDecideCallback = Arc::new(move |_content: String| {
            let attempts = Arc::clone(&attempts_for_cb);
            Box::pin(async move {
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    Err("LLM error".into())
                } else {
                    Ok(HeartbeatDecision {
                        action: "run".to_string(),
                        tasks: Some("Check logs".to_string()),
                    })
                }
            })
        });

        let execute_counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&execute_counter);
        let on_execute: HeartbeatExecuteCallback = Arc::new(move |tasks: String| {
            let counter = Arc::clone(&counter_clone);
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                format!("executed: {}", tasks)
            })
        });

        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(),
            config_with_retry(30, 2, 1, 2),
            None,
            PresenceManager::with_defaults(),
            Some(on_decide),
            Some(on_execute),
        );

        let result = service.trigger_now().await.unwrap();
        assert_eq!(result, "executed: Check logs");
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        assert_eq!(execute_counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_heartbeat_trigger_now_exhausted_retries_skips_execute() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("HEARTBEAT.md"), "Check logs")
            .await
            .unwrap();

        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_cb = Arc::clone(&attempts);
        let on_decide: HeartbeatDecideCallback = Arc::new(move |_content: String| {
            let attempts = Arc::clone(&attempts_for_cb);
            Box::pin(async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err("LLM error".into())
            })
        });

        let execute_counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&execute_counter);
        let on_execute: HeartbeatExecuteCallback = Arc::new(move |_tasks: String| {
            let counter = Arc::clone(&counter_clone);
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                "done".to_string()
            })
        });

        let bus = MessageBus::new();
        let mut rx = bus.subscribe();
        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(),
            config_with_retry(30, 2, 1, 2),
            Some(bus),
            PresenceManager::with_defaults(),
            Some(on_decide),
            Some(on_execute),
        );

        let result = service.trigger_now().await.unwrap();
        assert!(result.starts_with("error: "));
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        assert_eq!(execute_counter.load(Ordering::SeqCst), 0);
        assert_eq!(
            rx.try_recv().unwrap(),
            AgentBusEvent::HeartbeatTriggered {
                state: "error".to_string(),
                tasks: String::new(),
            }
        );
    }

    #[tokio::test]
    async fn test_heartbeat_background_tick_shares_retry_semantics() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("HEARTBEAT.md"), "Check logs")
            .await
            .unwrap();

        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_cb = Arc::clone(&attempts);
        let on_decide: HeartbeatDecideCallback = Arc::new(move |_content: String| {
            let attempts = Arc::clone(&attempts_for_cb);
            Box::pin(async move {
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    Err("LLM error".into())
                } else {
                    Ok(HeartbeatDecision {
                        action: "run".to_string(),
                        tasks: Some("Check logs".to_string()),
                    })
                }
            })
        });

        let execute_counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&execute_counter);
        let on_execute: HeartbeatExecuteCallback = Arc::new(move |_tasks: String| {
            let counter = Arc::clone(&counter_clone);
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                "done".to_string()
            })
        });

        let handle = build_handle(
            temp_dir.path().to_path_buf(),
            config_with_retry(30, 2, 1, 2),
            None,
            PresenceManager::with_defaults(),
            Some(on_decide),
            Some(on_execute),
        );

        handle.tick().await.unwrap();
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        assert_eq!(execute_counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_heartbeat_skip_no_execute_called() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(
            temp_dir.path().join("HEARTBEAT.md"),
            "# Tasks\n\nCheck logs",
        )
        .await
        .unwrap();

        let decide_counter = Arc::new(AtomicUsize::new(0));
        let dc = Arc::clone(&decide_counter);
        let on_decide: HeartbeatDecideCallback = Arc::new(move |_content: String| {
            let dc = Arc::clone(&dc);
            Box::pin(async move {
                dc.fetch_add(1, Ordering::SeqCst);
                Ok(HeartbeatDecision {
                    action: "skip".to_string(),
                    tasks: None,
                })
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

        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(),
            config_with_interval(1),
            None,
            PresenceManager::with_defaults(),
            Some(on_decide),
            Some(on_execute),
        );
        service.start().await;
        tokio::time::sleep(Duration::from_millis(1500)).await;
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
        )
        .await
        .unwrap();

        let decide_counter = Arc::new(AtomicUsize::new(0));
        let dc = Arc::clone(&decide_counter);
        let on_decide: HeartbeatDecideCallback = Arc::new(move |_content: String| {
            let dc = Arc::clone(&dc);
            Box::pin(async move {
                dc.fetch_add(1, Ordering::SeqCst);
                Ok(HeartbeatDecision {
                    action: "skip".to_string(),
                    tasks: None,
                })
            })
        });

        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(),
            config_with_interval(1),
            None,
            PresenceManager::with_defaults(),
            Some(on_decide),
            None,
        );
        service.start().await;
        tokio::time::sleep(Duration::from_millis(1500)).await;
        service.stop().await;

        assert_eq!(decide_counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_heartbeat_malformed_decide_emits_error_and_skips_execute() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("HEARTBEAT.md"), "Do something")
            .await
            .unwrap();

        let on_decide: HeartbeatDecideCallback =
            Arc::new(|_content: String| Box::pin(async move { Err("LLM error".into()) }));

        let execute_counter = Arc::new(AtomicUsize::new(0));
        let ec = Arc::clone(&execute_counter);
        let on_execute: HeartbeatExecuteCallback = Arc::new(move |_tasks: String| {
            let ec = Arc::clone(&ec);
            Box::pin(async move {
                ec.fetch_add(1, Ordering::SeqCst);
                "done".to_string()
            })
        });

        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(),
            config_with_retry(1, 0, 1, 1),
            None,
            PresenceManager::with_defaults(),
            Some(on_decide),
            Some(on_execute),
        );

        service.start().await;
        tokio::time::sleep(Duration::from_millis(1500)).await;
        service.stop().await;

        assert_eq!(execute_counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_heartbeat_emits_bus_event() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("HEARTBEAT.md"), "Check logs")
            .await
            .unwrap();
        let bus = MessageBus::new();
        let mut rx = bus.subscribe();
        let service = HeartbeatService::new(
            temp_dir.path().to_path_buf(),
            HeartbeatConfig::default(),
            Some(bus),
            PresenceManager::with_defaults(),
            Some(run_decide("Check logs")),
            None,
        );

        let result = service.trigger_now().await.unwrap();
        assert_eq!(result, "run (no execute callback)");
        assert_eq!(
            rx.try_recv().unwrap(),
            AgentBusEvent::HeartbeatTriggered {
                state: "run".to_string(),
                tasks: "Check logs".to_string(),
            }
        );
    }

    #[test]
    fn distracted_and_gone_use_configured_presence_multiplier() {
        let heartbeat = config_with_interval(10);
        let presence = PresenceManager::new(PresenceConfig {
            active_timeout_s: 5,
            distracted_timeout_s: 30,
            gone_timeout_s: 120,
            distracted_heartbeat_multiplier: 4.0,
        });

        presence.simulate_elapsed(Duration::from_secs(6));
        presence.refresh();
        assert_eq!(presence.state(), PresenceState::Distracted);
        assert_eq!(
            effective_interval_for_presence(&heartbeat, &presence),
            Some(Duration::from_secs(40))
        );

        presence.simulate_elapsed(Duration::from_secs(31));
        presence.refresh();
        assert_eq!(presence.state(), PresenceState::Gone);
        assert_eq!(
            effective_interval_for_presence(&heartbeat, &presence),
            Some(Duration::from_secs(40))
        );
    }

    #[test]
    fn away_presence_suspends_heartbeat() {
        let heartbeat = config_with_interval(10);
        let presence = PresenceManager::new(PresenceConfig {
            active_timeout_s: 5,
            distracted_timeout_s: 30,
            gone_timeout_s: 120,
            distracted_heartbeat_multiplier: 4.0,
        });

        presence.simulate_elapsed(Duration::from_secs(121));
        presence.refresh();
        assert_eq!(presence.state(), PresenceState::Away);
        assert_eq!(effective_interval_for_presence(&heartbeat, &presence), None);
    }

    #[test]
    fn effective_interval_is_clamped_to_one_second() {
        let heartbeat = config_with_interval(1);
        assert_eq!(
            effective_interval_for_rhythm(&heartbeat, HeartbeatRhythm::Slow, 0.25),
            Some(Duration::from_secs(1))
        );
    }
}
