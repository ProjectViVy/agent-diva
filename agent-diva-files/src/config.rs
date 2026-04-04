//! Configuration for file management system

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default maximum file size (500MB for UI)
pub const DEFAULT_MAX_FILE_SIZE: u64 = 500 * 1024 * 1024;

/// Default maximum total storage size (10GB)
pub const DEFAULT_MAX_TOTAL_SIZE: u64 = 10 * 1024 * 1024 * 1024;

/// Default cleanup interval in seconds (1 hour)
pub const DEFAULT_CLEANUP_INTERVAL_SECS: u64 = 3600;

/// Configuration for the file management system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    /// Base path for file storage
    #[serde(default = "default_storage_path")]
    pub storage_path: PathBuf,

    /// Maximum size for a single file
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,

    /// Maximum total storage size
    #[serde(default = "default_max_total_size")]
    pub max_total_size: u64,

    /// Whether to enable deduplication
    #[serde(default = "default_true")]
    pub deduplication: bool,

    /// Cleanup strategy
    #[serde(default)]
    pub cleanup: CleanupConfig,

    /// Channel-specific configuration
    #[serde(default)]
    pub channels: ChannelConfigs,
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            storage_path: default_storage_path(),
            max_file_size: DEFAULT_MAX_FILE_SIZE,
            max_total_size: DEFAULT_MAX_TOTAL_SIZE,
            deduplication: true,
            cleanup: CleanupConfig::default(),
            channels: ChannelConfigs::default(),
        }
    }
}

impl FileConfig {
    /// Create configuration with custom storage path
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            storage_path: path.into(),
            ..Default::default()
        }
    }

    /// Get the data directory path
    pub fn data_dir(&self) -> PathBuf {
        self.storage_path.join("data")
    }

    /// Get the index file path
    pub fn index_path(&self) -> PathBuf {
        self.storage_path.join("index.jsonl")
    }

    /// Get the config file path
    pub fn config_path(&self) -> PathBuf {
        self.storage_path.join("config.json")
    }
}

/// Cleanup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupConfig {
    /// Cleanup strategy
    #[serde(default)]
    pub strategy: CleanupStrategy,

    /// Maximum age in days before a file can be cleaned up
    #[serde(default = "default_max_age_days")]
    pub max_age_days: u32,

    /// Minimum reference count for cleanup eligibility
    #[serde(default)]
    pub min_ref_count: i32,

    /// Cleanup interval in seconds
    #[serde(default = "default_cleanup_interval")]
    pub interval_secs: u64,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            strategy: CleanupStrategy::Lazy,
            max_age_days: 7,
            min_ref_count: 0,
            interval_secs: DEFAULT_CLEANUP_INTERVAL_SECS,
        }
    }
}

/// Cleanup strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CleanupStrategy {
    /// Clean up immediately when ref_count reaches 0
    Immediate,
    /// Mark for deletion but delay actual cleanup
    #[default]
    Lazy,
    /// Only clean up on scheduled runs
    Scheduled,
}

/// Channel-specific configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfigs {
    #[serde(default)]
    pub telegram: ChannelConfig,
    #[serde(default)]
    pub discord: ChannelConfig,
    #[serde(default)]
    pub slack: ChannelConfig,
    #[serde(default)]
    pub ui: UiChannelConfig,
}

impl Default for ChannelConfigs {
    fn default() -> Self {
        Self {
            telegram: ChannelConfig {
                enabled: true,
                max_file_size: 10 * 1024 * 1024, // 10MB
                auto_download: true,
                preview_max_size: 100 * 1024, // 100KB
            },
            discord: ChannelConfig {
                enabled: true,
                max_file_size: 20 * 1024 * 1024, // 20MB
                auto_download: true,
                preview_max_size: 100 * 1024,
            },
            slack: ChannelConfig {
                enabled: true,
                max_file_size: 50 * 1024 * 1024, // 50MB
                auto_download: true,
                preview_max_size: 100 * 1024,
            },
            ui: UiChannelConfig {
                enabled: true,
                max_file_size: 500 * 1024 * 1024, // 500MB
                chunk_size: 10 * 1024 * 1024,     // 10MB chunks
                stream_upload: true,
                auto_download: true,
                preview_max_size: 1024 * 1024, // 1MB
            },
        }
    }
}

/// Generic channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelConfig {
    pub enabled: bool,
    pub max_file_size: u64,
    pub auto_download: bool,
    pub preview_max_size: u64,
}

/// UI-specific channel configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiChannelConfig {
    pub enabled: bool,
    pub max_file_size: u64,
    pub chunk_size: u64,
    pub stream_upload: bool,
    pub auto_download: bool,
    pub preview_max_size: u64,
}

// Default value functions
fn default_storage_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".agent-diva").join("files"))
        .unwrap_or_else(|| PathBuf::from(".agent-diva/files"))
}

fn default_max_file_size() -> u64 {
    DEFAULT_MAX_FILE_SIZE
}

fn default_max_total_size() -> u64 {
    DEFAULT_MAX_TOTAL_SIZE
}

fn default_max_age_days() -> u32 {
    7
}

fn default_cleanup_interval() -> u64 {
    DEFAULT_CLEANUP_INTERVAL_SECS
}

fn default_true() -> bool {
    true
}
