//! File management system for Agent Diva
//!
//! Provides content-addressed storage with deduplication and reference counting.
//! Files are stored by their SHA256 hash, enabling automatic deduplication.
//! Reference counting ensures files are only deleted when no longer referenced.
//!
//! ## 模块组织
//!
//! - [`backend`] - 存储后端 trait 和本地实现
//! - [`config`] - 配置管理
//! - [`handle`] - FileHandle 和元数据类型
//! - [`hooks`] - 生命周期钩子系统（压缩、加密、权限检查等）
//! - [`index`] - SQLite 索引实现
//! - [`manager`] - FileManager 主接口
//! - [`storage`] - 文件存储和 SHA256 计算

pub mod backend;
pub mod channel;
pub mod config;
pub mod handle;
pub mod hooks;
pub mod index;
pub mod manager;
pub mod storage;

// ---------------------------------------------------------------------------
// 公开导出 - Public Exports
// ---------------------------------------------------------------------------

/// 存储后端 trait 和实现
pub use backend::{BackendStats, LocalStorageBackend, StorageBackend};

/// 通道(Channel)文件管理
pub use channel::{ChannelFileInfo, ChannelManager, ChannelStats};

/// 文件管理配置
pub use config::{default_data_dir, default_data_dir_or_fallback, CleanupConfig, CleanupStrategy, FileConfig};

/// 文件句柄（用于引用存储的文件）
pub use handle::FileHandle;

/// 钩子系统的公共类型
// 导出 HookAction、HookRegistry 和所有 Hook trait
pub use hooks::{
    CleanupHook,  // 清理钩子 trait
    HookAction,   // 钩子返回值类型
    HookCounts,   // 钩子统计
    HookRegistry, // 钩子注册器
    MetadataHook, // 元数据钩子 trait
    ReadHook,     // 读取钩子 trait
    StorageHook,  // 存储钩子 trait
};

/// SQLite 索引实现
pub use index::{FileIndex, IndexStats, SqliteIndex};

/// FileManager 主接口
pub use manager::FileManager;

/// 文件存储统计
pub use storage::{FileStorage, StorageStats};

use thiserror::Error;

/// Errors that can occur in the file management system
#[derive(Error, Debug)]
pub enum FileError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("File not found: {0}")]
    NotFound(String),

    #[error("Invalid handle: {0}")]
    InvalidHandle(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("File too large: {0} bytes (max: {1} bytes)")]
    TooLarge(u64, u64),

    #[error("Hash mismatch: expected {0}, got {1}")]
    HashMismatch(String, String),

    #[error("Database error: {0}")]
    Database(String),
}

pub type Result<T> = std::result::Result<T, FileError>;

impl From<sqlx::Error> for FileError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => {
                FileError::NotFound("Record not found in database".to_string())
            }
            _ => FileError::Database(err.to_string()),
        }
    }
}

impl From<chrono::ParseError> for FileError {
    fn from(err: chrono::ParseError) -> Self {
        FileError::Storage(format!("Date parsing error: {}", err))
    }
}
