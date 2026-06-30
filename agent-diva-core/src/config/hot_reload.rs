//! Configuration hot-reload support
//!
//! Provides traits and utilities for hot-reloading configuration without
//! restarting the application. Only specific fields that are safe to update
//! at runtime are eligible for hot-reload.

use crate::config::schema::Config;
use crate::config::validate::validate_config;
use crate::config::ReloadPlan;
use crate::presence::PresenceConfig;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{watch, RwLock};
use tracing::{debug, error, info, warn};

// ---------------------------------------------------------------------------
// Hot-reloadable field registry
// ---------------------------------------------------------------------------

/// Fields that can be safely updated at runtime without restart.
///
/// Each variant represents a configuration section or individual field
/// that supports hot-reload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotReloadableField {
    /// PII detection rules (regex patterns, severity levels).
    PiiRules,
    /// Prompt injection detection patterns.
    InjectionPatterns,
    /// Presence state machine thresholds.
    PresenceThresholds,
    /// Log level (trace, debug, info, warn, error).
    LogLevel,
    /// Default tool execution timeout in seconds.
    ToolTimeout,
    /// MCP server configuration.
    McpServers,
    /// MCP manager configuration.
    McpManager,
}

impl HotReloadableField {
    /// Returns all hot-reloadable fields.
    pub fn all() -> HashSet<Self> {
        HashSet::from([
            Self::PiiRules,
            Self::InjectionPatterns,
            Self::PresenceThresholds,
            Self::LogLevel,
            Self::ToolTimeout,
            Self::McpServers,
            Self::McpManager,
        ])
    }

    /// Human-readable name for logging.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PiiRules => "pii_rules",
            Self::InjectionPatterns => "injection_patterns",
            Self::PresenceThresholds => "presence_thresholds",
            Self::LogLevel => "log_level",
            Self::ToolTimeout => "tool_timeout",
            Self::McpServers => "mcp_servers",
            Self::McpManager => "mcp_manager",
        }
    }
}

// ---------------------------------------------------------------------------
// HotReloadable trait
// ---------------------------------------------------------------------------

/// Trait for modules that can accept configuration updates at runtime.
///
/// Modules implementing this trait will be notified when hot-reloadable
/// configuration fields change. The module is responsible for applying
/// the new configuration to its internal state.
pub trait HotReloadable: Send + Sync {
    /// Returns the set of fields this module cares about.
    ///
    /// The reload system will only call [`on_config_reload`] when one
    /// of these fields has changed.
    fn watched_fields(&self) -> HashSet<HotReloadableField>;

    /// Called when one or more watched fields have changed.
    ///
    /// The `changed` set contains only the fields that actually changed
    /// since the last reload. The module should update its internal state
    /// accordingly.
    ///
    /// Returns `Ok(())` if the reload succeeded, or an error if the
    /// new configuration is invalid for this module.
    fn on_config_reload(
        &mut self,
        config: &Config,
        changed: &HashSet<HotReloadableField>,
    ) -> crate::Result<()>;
}

// ---------------------------------------------------------------------------
// Config delta detection
// ---------------------------------------------------------------------------

/// Compute which hot-reloadable fields changed between two configs.
pub fn compute_changed_fields(old: &Config, new: &Config) -> HashSet<HotReloadableField> {
    let mut changed = HashSet::new();
    if let Ok(plan) = ReloadPlan::from_configs(old, new) {
        for path in plan.diff.hot_reload_changes {
            match path.as_str() {
                "logging.level" => {
                    changed.insert(HotReloadableField::LogLevel);
                }
                "tools.exec.timeout" => {
                    changed.insert(HotReloadableField::ToolTimeout);
                }
                path if path.starts_with("presence.") || path.starts_with("heartbeat.") => {
                    changed.insert(HotReloadableField::PresenceThresholds);
                }
                path if path.starts_with("pii.") => {
                    changed.insert(HotReloadableField::PiiRules);
                }
                path if path.starts_with("injection.") => {
                    changed.insert(HotReloadableField::InjectionPatterns);
                }
                _ => {}
            }
        }
    }
    changed
}

// ---------------------------------------------------------------------------
// Config change event
// ---------------------------------------------------------------------------

/// Event emitted when configuration changes are detected.
#[derive(Debug, Clone)]
pub struct ConfigChangeEvent {
    /// The new configuration after reload.
    pub config: Config,
    /// Which hot-reloadable fields changed.
    pub changed_fields: HashSet<HotReloadableField>,
    /// Timestamp when the change was detected.
    pub detected_at: SystemTime,
}

// ---------------------------------------------------------------------------
// ConfigWatcher
// ---------------------------------------------------------------------------

/// Watches a configuration file for changes and triggers hot-reload.
///
/// Uses polling to detect file modification time changes, which is
/// portable across platforms and doesn't require additional dependencies.
pub struct ConfigWatcher {
    /// Path to the configuration file.
    config_path: PathBuf,
    /// Current configuration state.
    current_config: Arc<RwLock<Config>>,
    /// Last known modification time of the config file.
    last_modified: Arc<RwLock<Option<SystemTime>>>,
    /// Poll interval for checking file changes.
    poll_interval: Duration,
    /// Registered reloadable modules.
    modules: Arc<RwLock<Vec<Box<dyn HotReloadable>>>>,
    /// Channel for broadcasting config change events.
    change_tx: watch::Sender<Option<ConfigChangeEvent>>,
    /// Receiver for config change events (kept alive for subscribers).
    _change_rx: watch::Receiver<Option<ConfigChangeEvent>>,
}

impl ConfigWatcher {
    /// Create a new ConfigWatcher for the given config file.
    ///
    /// # Arguments
    /// * `config_path` - Path to the configuration file to watch.
    /// * `initial_config` - The initial loaded configuration.
    pub fn new(config_path: PathBuf, initial_config: Config) -> Self {
        let (change_tx, change_rx) = watch::channel(None);

        Self {
            config_path,
            current_config: Arc::new(RwLock::new(initial_config)),
            last_modified: Arc::new(RwLock::new(None)),
            poll_interval: Duration::from_secs(5), // Default: check every 5 seconds
            modules: Arc::new(RwLock::new(Vec::new())),
            change_tx,
            _change_rx: change_rx,
        }
    }

    /// Set the poll interval for file change detection.
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Register a module for hot-reload notifications.
    pub async fn register_module(&self, module: Box<dyn HotReloadable>) {
        let mut modules = self.modules.write().await;
        info!(
            module = ?module.watched_fields(),
            "Registered module for hot-reload"
        );
        modules.push(module);
    }

    /// Get a subscriber for config change events.
    pub fn subscribe(&self) -> watch::Receiver<Option<ConfigChangeEvent>> {
        self.change_tx.subscribe()
    }

    /// Get the current configuration.
    pub async fn current_config(&self) -> Config {
        self.current_config.read().await.clone()
    }

    /// Start the file watcher loop.
    ///
    /// This spawns a background task that polls the config file for changes.
    /// When a change is detected, it reloads the config, computes deltas,
    /// and notifies registered modules.
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!(
                path = %self.config_path.display(),
                interval = ?self.poll_interval,
                "Starting config file watcher"
            );

            // Initialize last_modified from current file state
            if let Ok(metadata) = std::fs::metadata(&self.config_path) {
                if let Ok(modified) = metadata.modified() {
                    *self.last_modified.write().await = Some(modified);
                }
            }

            let mut interval = tokio::time::interval(self.poll_interval);

            loop {
                interval.tick().await;

                if let Err(e) = self.check_and_reload().await {
                    error!(error = %e, "Failed to check/reload config");
                }
            }
        })
    }

    /// Check if the config file has changed and reload if necessary.
    async fn check_and_reload(&self) -> crate::Result<()> {
        let current_mtime = match std::fs::metadata(&self.config_path) {
            Ok(metadata) => metadata.modified().ok(),
            Err(_) => return Ok(()), // File doesn't exist or can't be read
        };

        let last_mtime = *self.last_modified.read().await;

        // Check if file was modified
        if current_mtime == last_mtime {
            return Ok(());
        }

        info!("Config file change detected, reloading...");

        // Read and parse the new config
        let content = std::fs::read_to_string(&self.config_path)?;
        let new_config: Config = match serde_json::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                error!(error = %e, "Failed to parse config file, keeping current config");
                // Update mtime to avoid re-parsing the same bad file
                *self.last_modified.write().await = current_mtime;
                return Ok(());
            }
        };

        // Validate the new config
        if let Err(e) = validate_config(&new_config) {
            error!(error = %e, "New config failed validation, keeping current config");
            *self.last_modified.write().await = current_mtime;
            return Ok(());
        }

        let old_config = self.current_config.read().await.clone();
        let reload_plan = ReloadPlan::from_configs(&old_config, &new_config)?;
        if !reload_plan.diff.restart_required_changes.is_empty() {
            warn!(
                restart_required = ?reload_plan.diff.restart_required_changes,
                "Config change requires restart; skipping live reload"
            );
            *self.last_modified.write().await = current_mtime;
            return Ok(());
        }

        // Compute which fields changed
        let changed = compute_changed_fields(&old_config, &new_config);

        if changed.is_empty() {
            debug!("Config file changed but no hot-reloadable fields affected");
            *self.last_modified.write().await = current_mtime;
            *self.current_config.write().await = new_config;
            return Ok(());
        }

        info!(changed = ?changed, "Hot-reloadable fields changed");

        // Notify registered modules
        let mut modules = self.modules.write().await;
        let mut all_succeeded = true;

        for module in modules.iter_mut() {
            let module_fields = module.watched_fields();
            let module_changed: HashSet<_> =
                changed.intersection(&module_fields).copied().collect();

            if module_changed.is_empty() {
                continue;
            }

            match module.on_config_reload(&new_config, &module_changed) {
                Ok(()) => {
                    debug!(fields = ?module_changed, "Module reloaded successfully");
                }
                Err(e) => {
                    error!(error = %e, fields = ?module_changed, "Module failed to reload");
                    all_succeeded = false;
                }
            }
        }

        if all_succeeded {
            // Update current config and mtime
            *self.current_config.write().await = new_config.clone();
            *self.last_modified.write().await = current_mtime;

            // Broadcast change event
            let event = ConfigChangeEvent {
                config: new_config,
                changed_fields: changed,
                detected_at: SystemTime::now(),
            };
            let _ = self.change_tx.send(Some(event));
        } else {
            warn!("Some modules failed to reload, config not fully updated");
            // Still update mtime to avoid re-processing the same file
            *self.last_modified.write().await = current_mtime;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PresenceConfig hot-reload helper
// ---------------------------------------------------------------------------

/// Extract presence configuration from the main config.
///
/// This provides a bridge between the main Config struct and the
/// presence module's configuration format.
pub fn extract_presence_config(config: &Config) -> PresenceConfig {
    config.presence.clone()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::Config;
    use std::collections::HashSet;
    use tempfile::TempDir;

    /// A test module that tracks reload calls.
    struct TestModule {
        watched: HashSet<HotReloadableField>,
        reload_count: usize,
        last_changed: HashSet<HotReloadableField>,
        fail_reload: bool,
    }

    impl TestModule {
        fn new(watched: HashSet<HotReloadableField>) -> Self {
            Self {
                watched,
                reload_count: 0,
                last_changed: HashSet::new(),
                fail_reload: false,
            }
        }

        fn failing(watched: HashSet<HotReloadableField>) -> Self {
            Self {
                watched,
                reload_count: 0,
                last_changed: HashSet::new(),
                fail_reload: true,
            }
        }
    }

    impl HotReloadable for TestModule {
        fn watched_fields(&self) -> HashSet<HotReloadableField> {
            self.watched.clone()
        }

        fn on_config_reload(
            &mut self,
            _config: &Config,
            changed: &HashSet<HotReloadableField>,
        ) -> crate::Result<()> {
            if self.fail_reload {
                return Err(crate::Error::Internal("forced reload failure".to_string()));
            }
            self.reload_count += 1;
            self.last_changed = changed.clone();
            Ok(())
        }
    }

    #[test]
    fn hot_reloadable_field_all_contains_expected() {
        let all = HotReloadableField::all();
        assert!(all.contains(&HotReloadableField::PiiRules));
        assert!(all.contains(&HotReloadableField::InjectionPatterns));
        assert!(all.contains(&HotReloadableField::PresenceThresholds));
        assert!(all.contains(&HotReloadableField::LogLevel));
        assert!(all.contains(&HotReloadableField::ToolTimeout));
        assert!(all.contains(&HotReloadableField::McpServers));
        assert!(all.contains(&HotReloadableField::McpManager));
        assert_eq!(all.len(), 7);
    }

    #[test]
    fn hot_reloadable_field_as_str_returns_readable_names() {
        assert_eq!(HotReloadableField::PiiRules.as_str(), "pii_rules");
        assert_eq!(HotReloadableField::LogLevel.as_str(), "log_level");
        assert_eq!(HotReloadableField::ToolTimeout.as_str(), "tool_timeout");
        assert_eq!(HotReloadableField::McpServers.as_str(), "mcp_servers");
        assert_eq!(HotReloadableField::McpManager.as_str(), "mcp_manager");
    }

    #[test]
    fn compute_changed_fields_detects_log_level_change() {
        let old = Config::default();
        let mut new = Config::default();
        new.logging.level = "debug".to_string();

        let changed = compute_changed_fields(&old, &new);
        assert!(changed.contains(&HotReloadableField::LogLevel));
        assert_eq!(changed.len(), 1);
    }

    #[test]
    fn compute_changed_fields_detects_tool_timeout_change() {
        let old = Config::default();
        let mut new = Config::default();
        new.tools.exec.timeout = 120;

        let changed = compute_changed_fields(&old, &new);
        assert!(changed.contains(&HotReloadableField::ToolTimeout));
        assert_eq!(changed.len(), 1);
    }

    #[test]
    fn compute_changed_fields_detects_multiple_changes() {
        let old = Config::default();
        let mut new = Config::default();
        new.logging.level = "debug".to_string();
        new.tools.exec.timeout = 120;

        let changed = compute_changed_fields(&old, &new);
        assert!(changed.contains(&HotReloadableField::LogLevel));
        assert!(changed.contains(&HotReloadableField::ToolTimeout));
        assert_eq!(changed.len(), 2);
    }

    #[test]
    fn compute_changed_fields_returns_empty_for_same_config() {
        let config = Config::default();
        let changed = compute_changed_fields(&config, &config);
        assert!(changed.is_empty());
    }

    #[test]
    fn test_module_tracks_reload_calls() {
        let mut module = TestModule::new(HotReloadableField::all());
        assert_eq!(module.reload_count, 0);

        let config = Config::default();
        let mut changed = HashSet::new();
        changed.insert(HotReloadableField::LogLevel);

        module.on_config_reload(&config, &changed).unwrap();
        assert_eq!(module.reload_count, 1);
        assert!(module.last_changed.contains(&HotReloadableField::LogLevel));
    }

    #[tokio::test]
    async fn config_watcher_detects_file_changes() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Write initial config
        let initial_config = Config::default();
        let content = serde_json::to_string_pretty(&initial_config).unwrap();
        std::fs::write(&config_path, &content).unwrap();

        // Create watcher
        let watcher = ConfigWatcher::new(config_path.clone(), initial_config.clone())
            .with_poll_interval(Duration::from_millis(100));

        // Register a test module
        let module = TestModule::new(HotReloadableField::all());
        watcher.register_module(Box::new(module)).await;

        // Start watching
        let watcher = Arc::new(watcher);
        let handle = watcher.clone().start();

        // Give watcher time to initialize
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Modify config file
        let mut new_config = initial_config.clone();
        new_config.logging.level = "debug".to_string();
        let content = serde_json::to_string_pretty(&new_config).unwrap();
        std::fs::write(&config_path, &content).unwrap();

        // Wait for watcher to detect change
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Check that config was updated
        let current = watcher.current_config().await;
        assert_eq!(current.logging.level, "debug");

        // Cleanup
        handle.abort();
    }

    #[tokio::test]
    async fn config_watcher_keeps_old_config_when_module_reload_fails() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let initial_config = Config::default();
        std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&initial_config).unwrap(),
        )
        .unwrap();

        let watcher = ConfigWatcher::new(config_path.clone(), initial_config.clone())
            .with_poll_interval(Duration::from_millis(10));
        watcher
            .register_module(Box::new(TestModule::failing(HotReloadableField::all())))
            .await;

        let mut new_config = initial_config.clone();
        new_config.logging.level = "debug".to_string();
        std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&new_config).unwrap(),
        )
        .unwrap();

        watcher.check_and_reload().await.unwrap();

        let current = watcher.current_config().await;
        assert_eq!(current.logging.level, initial_config.logging.level);
    }

    #[tokio::test]
    async fn config_watcher_skips_restart_required_changes() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        let initial_config = Config::default();
        std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&initial_config).unwrap(),
        )
        .unwrap();

        let watcher = ConfigWatcher::new(config_path.clone(), initial_config.clone())
            .with_poll_interval(Duration::from_millis(10));

        let mut new_config = initial_config.clone();
        new_config.gateway.port += 1;
        std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&new_config).unwrap(),
        )
        .unwrap();

        watcher.check_and_reload().await.unwrap();

        let current = watcher.current_config().await;
        assert_eq!(current.gateway.port, initial_config.gateway.port);
    }

    #[test]
    fn extract_presence_config_returns_configured_values() {
        let config = Config::default();
        let presence_config = extract_presence_config(&config);

        assert_eq!(presence_config, config.presence);
    }

    #[test]
    fn compute_changed_fields_detects_harness_domain_rule_changes() {
        let old = Config::default();
        let mut new = old.clone();
        new.presence.active_timeout_s += 1;
        new.pii.redact_email = !new.pii.redact_email;
        new.injection.detect_tool_abuse = !new.injection.detect_tool_abuse;

        let changed = compute_changed_fields(&old, &new);

        assert!(changed.contains(&HotReloadableField::PresenceThresholds));
        assert!(changed.contains(&HotReloadableField::PiiRules));
        assert!(changed.contains(&HotReloadableField::InjectionPatterns));
    }
}
