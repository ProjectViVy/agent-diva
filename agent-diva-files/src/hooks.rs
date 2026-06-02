//! # Hook System - 钩子系统的全面解析
//!
//! 本模块实现了一套完整的生命周期钩子系统，允许开发者在文件操作的各个阶段
//! 插入自定义逻辑，实现诸如压缩、加密、权限检查、缓存等功能。
//!
//! ## 钩子系统架构
//!
//! 钩子系统由两部分组成：**钩子 trait** 和 **钩子注册器**。
//!
//! ### 1. 钩子 Trait（Hooks）
//!
//! 每种钩子 trait 定义了一个特定的扩展点：
//!
//! | Trait | 扩展点 | 用途 |
//! |-------|--------|------|
//! | [`StorageHook`] | 存储前/后 | 压缩、加密、病毒扫描、通知 |
//! | [`ReadHook`] | 读取前/后 | 权限检查、解密、缓存 |
//! | [`MetadataHook`] | 元数据提取/验证 | 自定义元数据提取、内容分析 |
//! | [`CleanupHook`] | 清理前/后 | 自定义保留策略、日志记录 |
//!
//! ### 2. 钩子注册器（HookRegistry）
//!
//! [`HookRegistry`] 是中央注册表，负责：
//! - 存储所有注册的钩子
//! - 按照预定义的顺序执行钩子
//! - 处理钩子返回的 [`HookAction`]（继续、阻止、重试）
//!
//! ## 执行流程
//!
//! ### 存储流程
//! ```text
//! store(data, metadata)
//!   └─→ StorageHook::before_store (所有注册的钩子串行执行)
//!         ├─→ Hook 1: 可以修改数据、返回错误阻止存储
//!         ├─→ Hook 2: ...
//!         └─→ Hook N: ...
//!   └─→ [数据存储到磁盘]
//!   └─→ MetadataHook::extract_metadata (提取额外元数据)
//!   └─→ [写入索引数据库]
//!   └─→ StorageHook::after_store (所有注册的钩子串行执行)
//!         └─→ 通知、缓存更新等
//! ```
//!
//! ### 读取流程
//! ```text//! read(handle)
//!   └─→ ReadHook::before_read (权限检查等)
//!         ├─→ Hook 1: 检查权限、返回错误阻止读取
//!         └─→ Hook 2: ...
//!   └─→ [从磁盘读取数据]
//!   └─→ ReadHook::after_read (数据转换)
//!         ├─→ Hook 1: 解密数据
//!         └─→ Hook 2: 缓存数据、转换格式
//! ```
//!
//! ### 清理流程
//! ```text
//! cleanup()
//!   └─→ 对于每个候选文件:
//!         ├─→ CleanupHook::should_cleanup (任一返回 false 则跳过)
//!         └─→ [物理删除文件]
//!         └─→ CleanupHook::after_cleanup (日志、通知)
//! ```
//!
//! ## 使用示例
//!
//! ### 示例1: 简单的日志钩子
//!
//! ```rust,ignore
//! use agent_diva_files::hooks::{HookRegistry, StorageHook, HookAction};
//! use agent_diva_files::{FileMetadata, Result};
//! use async_trait::async_trait;
//!
//! struct LoggingHook;
//!
//! #[async_trait]
//! impl StorageHook for LoggingHook {
//!     async fn before_store(&self, data: &[u8], metadata: &FileMetadata) -> Result<HookAction> {
//!         println!("Storing file: {}", metadata.name);
//!         Ok(HookAction::Continue)  // 允许存储继续
//!     }
//!
//!     async fn after_store(&self, handle: &FileHandle) -> Result<()> {
//!         println!("Stored file: {}", handle.id);
//!         Ok(())
//!     }
//! }
//!
//! // 注册钩子
//! let mut registry = HookRegistry::new();
//! registry.register_storage_hook(Box::new(LoggingHook));
//! ```
//!
//! ### 示例2: 加密钩子
//!
//! ```rust,ignore
//! use agent_diva_files::hooks::{HookRegistry, StorageHook, HookAction};
//! use agent_diva_files::{FileMetadata, Result};
//! use async_trait::async_trait;
//!
//! struct EncryptHook {
//!     key: Vec<u8>,
//! }
//!
//! #[async_trait]
//! impl StorageHook for EncryptHook {
//!     // 在存储前加密数据
//!     async fn before_store(&self, data: &[u8], _metadata: &FileMetadata) -> Result<HookAction> {
//!         let encrypted = encrypt(data, &self.key)?;
//!         // 返回 Modify 告诉系统使用修改后的数据
//!         Ok(HookAction::Modify(encrypted))
//!     }
//! }
//! ```
//!
//! ### 示例3: 权限检查钩子
//!
//! ```rust,ignore
//! use agent_diva_files::hooks::{HookRegistry, ReadHook, HookAction};
//! use agent_diva_files::Result;
//! use async_trait::async_trait;
//!
//! struct PermissionCheck {
//!     allowed_users: Vec<String>,
//! }
//!
//! #[async_trait]
//! impl ReadHook for PermissionCheck {
//!     async fn before_read(&self, id: &str, requester: Option<&str>) -> Result<HookAction> {
//!         match requester {
//!             Some(user) if self.allowed_users.contains(&user.to_string()) => {
//!                 Ok(HookAction::Continue)
//!             }
//!             _ => Err(FileError::Storage("Permission denied".to_string())),
//!         }
//!     }
//! }
//! ```

use crate::handle::{FileHandle, FileMetadata};
use crate::Result;
use async_trait::async_trait;

// ============================================================================
// 钩子动作 - HookAction
// ============================================================================

/// 钩子执行后返回的动作指令
///
/// 当一个钩子执行完毕后，它返回一个 `HookAction` 来告诉系统如何继续处理。
/// 系统会按照以下优先级处理：
/// 1. 如果任何 `before_*` 钩子返回 `Stop`，整个操作被阻止
/// 2. 如果任何 `before_*` 钩子返回 `Error`，整个操作失败
/// 3. 只有所有 `before_*` 钩子都返回 `Continue` 或 `Modify` 时，操作才会继续
///
/// # 变体说明
///
/// - `Continue`: 继续执行，不修改数据。这是大多数钩子的默认返回值。
///
/// - `Modify(data)`: 继续执行，但使用修改后的数据。
///   只有 `before_store` 和 `after_read` 支持此变体。
///   例如：加密钩子在 `before_store` 返回 `Modify(encrypted_data)`
///
/// - `Stop`: 立即停止执行，但不算错误。
///   例如：缓存钩子发现数据已在缓存中，返回 `Stop` 避免重复读取
///
/// - `Error(err)`: 立即停止并返回错误。
///   例如：权限检查发现无权访问时返回此值
#[derive(Debug, Clone)]
pub enum HookAction {
    /// 继续执行下一个钩子或操作
    Continue,

    /// 继续执行，但使用修改后的数据
    /// (仅用于 before_store 和 after_read)
    Modify(Vec<u8>),

    /// 立即停止执行（不算错误）
    Stop,

    /// 立即停止并返回错误
    Error(String),
}

impl HookAction {
    /// 判断是否应该继续执行
    ///
    /// 注意：`Modify` 也会继续执行，只是会携带修改后的数据
    pub fn should_continue(&self) -> bool {
        matches!(self, HookAction::Continue | HookAction::Modify(_))
    }

    /// 判断是否携带了修改后的数据
    pub fn has_modified_data(&self) -> bool {
        matches!(self, HookAction::Modify(_))
    }

    /// 获取修改后的数据（如果有）
    pub fn get_modified_data(self) -> Option<Vec<u8>> {
        match self {
            HookAction::Modify(data) => Some(data),
            _ => None,
        }
    }
}

// ============================================================================
// 存储钩子 - StorageHook
// ============================================================================

/// 存储钩子 - 拦截文件存储操作
///
/// 在文件存储到磁盘之前和之后执行自定义逻辑。
///
/// # 使用场景
///
/// - **数据压缩**: 在存储前压缩数据，节省空间
/// - **数据加密**: 在存储前加密敏感数据
/// - **病毒扫描**: 在存储前扫描恶意软件
/// - **内容审核**: 检查内容是否符合政策
/// - **存储通知**: 记录文件存储事件用于审计
///
/// # 执行时机
///
/// 1. `before_store` 在数据写入磁盘**之前**执行
///    - 可以修改要存储的数据
///    - 可以拒绝存储（返回 `HookAction::Error`）
///    - 如果返回 `HookAction::Modify`，系统会存储修改后的数据
///
/// 2. `after_store` 在数据成功写入磁盘**之后**执行
///    - 不能修改数据（此时数据已写入）
///    - 适合做通知、缓存更新等操作
///
/// # 错误处理
///
/// 如果 `before_store` 返回错误，文件**不会被存储**，系统会立即返回错误。
/// 如果 `after_store` 返回错误，文件**已经被存储**，错误会被记录但不会回滚。
///
/// # 示例：压缩钩子
///
/// ```rust,ignore
/// use async_trait::async_trait;
/// use agent_diva_files::hooks::{StorageHook, HookAction};
/// use agent_diva_files::{FileMetadata, Result};
///
/// struct CompressionHook {
///
///     #[async_trait]
///     impl StorageHook for CompressionHook {
///         async fn before_store(&self, data: &[u8], metadata: &FileMetadata) -> Result<HookAction> {
///             // 只压缩大于1MB的文件
///             if data.len() > 1024 * 1024 {
///                 let compressed = compress(data)?;
///                 Ok(HookAction::Modify(compressed))
///             } else {
///                 Ok(HookAction::Continue)
///             }
///         }
///
///         async fn after_store(&self, handle: &FileHandle) -> Result<()> {
///             tracing::info!("Compressed file stored: {}", handle.id);
///             Ok(())
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait StorageHook: Send + Sync {
    /// 存储前钩子 - 在数据写入磁盘之前调用
    ///
    /// # 参数
    /// - `data`: 要存储的原始数据字节
    /// - `metadata`: 文件元数据（包含文件名、大小、MIME类型等）
    ///
    /// # 返回值
    /// - `Ok(HookAction::Continue)`: 继续存储，使用原始数据
    /// - `Ok(HookAction::Modify(data))`: 继续存储，但使用修改后的数据
    /// - `Ok(HookAction::Stop)`: 停止执行，但不报错（数据不会被存储）
    /// - `Ok(HookAction::Error(msg))`: 停止执行，返回错误
    ///
    /// # 默认实现
    /// 默认实现返回 `HookAction::Continue`，不在存储前做任何处理
    async fn before_store(&self, _data: &[u8], _metadata: &FileMetadata) -> Result<HookAction> {
        Ok(HookAction::Continue)
    }

    /// 存储后钩子 - 在数据成功写入磁盘之后调用
    ///
    /// # 参数
    /// - `handle`: 文件句柄，包含文件ID、路径和元数据
    ///
    /// # 返回值
    /// - `Ok(())`: 钩子执行成功
    /// - `Err(e)`: 钩子执行失败（会被记录但不会影响已存储的文件）
    ///
    /// # 默认实现
    /// 默认实现什么都不做，直接返回成功
    async fn after_store(&self, _handle: &FileHandle) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// 读取钩子 - ReadHook
// ============================================================================

/// 读取钩子 - 拦截文件读取操作
///
/// 在文件从磁盘读取之前和之后执行自定义逻辑。
///
/// # 使用场景
///
/// - **权限检查**: 验证请求者是否有权读取文件
/// - **数据解密**: 读取后解密加密的数据
/// - **缓存处理**: 读取后更新缓存、设置缓存策略
/// - **内容转换**: 读取后转换数据格式（如图片缩放）
/// - **访问统计**: 记录文件访问用于分析
///
/// # 执行时机
///
/// 1. `before_read` 在数据从磁盘读取**之前**执行
///    - 适合做权限检查
///    - 如果返回 `Stop`，可以避免实际的磁盘读取
///    - 如果返回 `Error`，读取操作失败
///
/// 2. `after_read` 在数据从磁盘读取**之后**执行
///    - 可以修改读取的数据
///    - 适合做解密、缓存等操作
///
/// # 与存储钩子的对称性
///
/// 存储钩子和读取钩子常常成对出现：
/// - `before_store` / `after_read`: 配对的加密/解密
/// - `after_store` / `before_read`: 配对的权限检查
///
/// # 示例：解密钩子
///
/// ```rust,ignore
/// use async_trait::async_trait;
/// use agent_diva_files::hooks::{ReadHook, HookAction};
/// use agent_diva_files::Result;
///
/// struct DecryptionHook {
///     key: Vec<u8>,
/// }
///
/// #[async_trait]
/// impl ReadHook for DecryptionHook {
///     async fn before_read(&self, _id: &str, _requester: Option<&str>) -> Result<HookAction> {
///         // 权限检查可以在这里进行
///         Ok(HookAction::Continue)
///     }
///
///     async fn after_read(&self, data: &[u8]) -> Result<HookAction> {
///         // 解密数据
///         let decrypted = decrypt(data, &self.key)?;
///         Ok(HookAction::Modify(decrypted))
///     }
/// }
/// ```
#[async_trait]
pub trait ReadHook: Send + Sync {
    /// 读取前钩子 - 在数据从磁盘读取之前调用
    ///
    /// # 参数
    /// - `id`: 文件的唯一标识符（SHA256哈希）
    /// - `requester`: 请求读取的用户标识（如果有）
    ///
    /// # 返回值
    /// - `Ok(HookAction::Continue)`: 继续读取
    /// - `Ok(HookAction::Stop)`: 停止执行（数据未读取）
    /// - `Ok(HookAction::Error(msg))`: 读取失败
    ///
    /// # 默认实现
    /// 默认实现不做任何检查，直接返回继续
    async fn before_read(&self, _id: &str, _requester: Option<&str>) -> Result<HookAction> {
        Ok(HookAction::Continue)
    }

    /// 读取后钩子 - 在数据从磁盘读取之后调用
    ///
    /// # 参数
    /// - `data`: 从磁盘读取的原始数据
    ///
    /// # 返回值
    /// - `Ok(HookAction::Continue)`: 使用原始数据
    /// - `Ok(HookAction::Modify(data))`: 使用修改后的数据
    /// - `Err(e)`: 处理失败
    ///
    /// # 默认实现
    /// 默认实现不做任何处理，直接返回原始数据
    async fn after_read(&self, data: &[u8]) -> Result<HookAction> {
        Ok(HookAction::Modify(data.to_vec()))
    }
}

// ============================================================================
// 元数据钩子 - MetadataHook
// ============================================================================

/// 元数据钩子 - 自定义元数据提取和验证
///
/// 允许在标准元数据之外提取额外的文件属性。
///
/// # 使用场景
///
/// - **内容分析**: 从文件内容中提取额外信息（如图片尺寸、文档页数）
/// - **标签生成**: 基于内容自动生成标签
/// - **格式检测**: 检测并记录精确的MIME类型
/// - **完整性验证**: 验证文件内容与声称的格式是否匹配
///
/// # 示例：图片尺寸提取钩子
///
/// ```rust,ignore
/// use async_trait::async_trait;
/// use agent_diva_files::hooks::MetadataHook;
/// use agent_diva_files::{FileMetadata, Result};
///
/// struct ImageDimensionsHook;
///
/// #[async_trait]
/// impl MetadataHook for ImageDimensionsHook {
///     async fn extract_metadata(&self, data: &[u8], base: &FileMetadata) -> Result<serde_json::Value> {
///         if let Some(dimensions) = extract_image_dimensions(data)? {
///             Ok(serde_json::json!({
///                 "width": dimensions.0,
///                 "height": dimensions.1,
///             }))
///         } else {
///             Ok(serde_json::Value::Null)
///         }
///     }
///
///     async fn validate_metadata(&self, metadata: &FileMetadata) -> Result<()> {
///         // 验证MIME类型与实际内容是否匹配
///         if metadata.name.ends_with(".png") && metadata.mime_type != Some("image/png".to_string()) {
///             return Err(FileError::Storage("MIME type mismatch".to_string()));
///         }
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait MetadataHook: Send + Sync {
    /// 提取额外元数据
    ///
    /// 在文件存储时调用，可以从内容中提取额外信息。
    ///
    /// # 参数
    /// - `data`: 文件的原始数据
    /// - `base`: 基础元数据（已有文件名、大小等）
    ///
    /// # 返回值
    /// 返回一个 JSON Value，会被合并到文件的 `metadata.extra` 字段
    ///
    /// # 默认实现
    /// 默认返回 `serde_json::Value::Null`，不添加额外信息
    async fn extract_metadata(
        &self,
        _data: &[u8],
        _base: &FileMetadata,
    ) -> Result<serde_json::Value> {
        Ok(serde_json::Value::Null)
    }

    /// 验证元数据
    ///
    /// 在文件存储前调用，验证元数据的有效性。
    ///
    /// # 参数
    /// - `metadata`: 要验证的元数据
    ///
    /// # 返回值
    /// - `Ok(())`: 验证通过
    /// - `Err(e)`: 验证失败，文件不会被存储
    ///
    /// # 默认实现
    /// 默认实现不做验证，总是返回成功
    async fn validate_metadata(&self, _metadata: &FileMetadata) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// 清理钩子 - CleanupHook
// ============================================================================

/// 清理钩子 - 自定义文件清理策略
///
/// 控制在什么条件下文件可以被清理删除。
///
/// # 使用场景
///
/// - **自定义保留策略**: 如只保留最近7天被访问过的文件
/// - **保护重要文件**: 标记某些文件为"永不清理"
/// - **清理前通知**: 在删除前发送通知
/// - **清理后处理**: 删除后清理相关缓存、关联数据
///
/// # 执行时机
///
/// 1. `should_cleanup` 在文件**实际删除之前**调用
///    - 返回 `true` 允许删除
///    - 返回 `false` 跳过此文件
///    - 返回错误也会阻止删除
///
/// 2. `after_cleanup` 在文件**实际删除之后**调用
///    - 常用于清理相关资源
///
/// # 重要：与软删除的关系
///
/// 清理钩子工作在**物理删除**层面：
/// - 软删除文件由 `soft_delete()` 处理，有自己的保留期
/// - 清理钩子只在文件**真正从数据库删除**时触发
///
/// # 示例：只清理超过30天未访问的文件
///
/// ```rust,ignore
/// use async_trait::async_trait;
/// use agent_diva_files::hooks::CleanupHook;
/// use agent_diva_files::{FileIndexEntry, Result};
/// use chrono::{DateTime, Utc};
///
/// struct CustomRetentionHook {
///     min_days_since_access: i64,
/// }
///
/// #[async_trait]
/// impl CleanupHook for CustomRetentionHook {
///     async fn should_cleanup(&self, entry: &FileIndexEntry) -> Result<bool> {
///         if let Some(last_access) = entry.last_accessed_at {
///             let days_since = (Utc::now() - last_access).num_days();
///             Ok(days_since > self.min_days_since_access)
///         } else {
///             // 从未被访问过，检查创建时间
///             let days_since = (Utc::now() - entry.created_at).num_days();
///             Ok(days_since > self.min_days_since_access)
///         }
///     }
/// }
/// ```
#[async_trait]
pub trait CleanupHook: Send + Sync {
    /// 清理前检查 - 决定是否应该删除文件
    ///
    /// # 参数
    /// - `entry`: 文件索引条目，包含文件的完整信息
    ///
    /// # 返回值
    /// - `Ok(true)`: 允许清理，系统会删除文件
    /// - `Ok(false)`: 跳过此文件，不删除
    /// - `Err(e)`: 阻止清理，返回错误
    ///
    /// # 默认实现
    /// 默认实现总是返回 `true`，允许清理所有候选文件
    async fn should_cleanup(&self, _entry: &crate::handle::FileIndexEntry) -> Result<bool> {
        Ok(true)
    }

    /// 清理后钩子 - 文件被删除后调用
    ///
    /// # 参数
    /// - `entry`: 被删除文件的原始条目信息
    ///
    /// # 返回值
    /// - `Ok(())`: 清理后处理成功
    /// - `Err(e)`: 清理后处理失败（会被记录但不会回滚删除）
    ///
    /// # 默认实现
    /// 默认实现什么都不做
    async fn after_cleanup(&self, _entry: &crate::handle::FileIndexEntry) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// 钩子注册器 - HookRegistry
// ============================================================================

/// 钩子注册器 - 统一管理所有类型的钩子
///
/// `HookRegistry` 是钩子系统的中央组件，负责：
/// - 存储所有注册的钩子实例
/// - 按顺序执行钩子
/// - 协调各类型钩子的执行
///
/// # 线程安全
///
/// `HookRegistry` 内部使用 `std::sync::Mutex` 保护状态，
/// 因此可以在多线程环境中安全使用。
///
/// # 示例：完整的钩子注册流程
///
/// ```rust,ignore
/// use agent_diva_files::hooks::{HookRegistry, StorageHook, ReadHook, CleanupHook};
/// use agent_diva_files::hooks::CompressionHook;
/// use agent_diva_files::FileManager;
///
/// // 创建注册器
/// let mut registry = HookRegistry::new();
///
/// // 注册存储钩子
/// registry.register_storage_hook(Box::new(CompressionHook::new()));
///
/// // 注册读取钩子
/// registry.register_read_hook(Box::new(DecryptionHook::new()));
///
/// // 注册清理钩子
/// registry.register_cleanup_hook(Box::new(RetentionPolicyHook::new()));
///
/// // 将注册器传给 FileManager
/// let manager = FileManager::new(config, registry).await?;
/// ```
///
/// # 执行顺序
///
/// 当多个同类型钩子注册时，它们按照**注册顺序**串行执行。
/// 前一个钩子的输出（修改后的数据）会传给下一个钩子作为输入。
///
/// 例如：注册了 [HookA, HookB]，存储流程是：
/// ```text
/// data → HookA.before_store → HookB.before_store → [存储]
/// ```
#[derive(Default)]
pub struct HookRegistry {
    /// 已注册的存储钩子列表
    storage_hooks: Vec<Box<dyn StorageHook>>,

    /// 已注册的读取钩子列表
    read_hooks: Vec<Box<dyn ReadHook>>,

    /// 已注册的元数据钩子列表
    metadata_hooks: Vec<Box<dyn MetadataHook>>,

    /// 已注册的清理钩子列表
    cleanup_hooks: Vec<Box<dyn CleanupHook>>,
}

impl std::fmt::Debug for HookRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookRegistry")
            .field("storage_hooks", &self.storage_hooks.len())
            .field("read_hooks", &self.read_hooks.len())
            .field("metadata_hooks", &self.metadata_hooks.len())
            .field("cleanup_hooks", &self.cleanup_hooks.len())
            .finish()
    }
}

impl HookRegistry {
    /// 创建新的空钩子注册器
    pub fn new() -> Self {
        Self::default()
    }

    // -------------------------------------------------------------------------
    // 注册方法
    // -------------------------------------------------------------------------

    /// 注册一个存储钩子
    ///
    /// 存储钩子会在文件存储时被调用。
    /// 可以注册多个存储钩子，它们会按注册顺序串行执行。
    ///
    /// # 参数
    /// - `hook`: 实现 `StorageHook` trait 的 Box 指针
    ///
    /// # 示例
    /// ```rust,ignore
    /// registry.register_storage_hook(Box::new(MyStorageHook));
    /// ```
    pub fn register_storage_hook(&mut self, hook: Box<dyn StorageHook>) {
        self.storage_hooks.push(hook);
    }

    /// 注册一个读取钩子
    ///
    /// 读取钩子会在文件读取时被调用。
    /// 可以注册多个读取钩子，它们会按注册顺序串行执行。
    pub fn register_read_hook(&mut self, hook: Box<dyn ReadHook>) {
        self.read_hooks.push(hook);
    }

    /// 注册一个元数据钩子
    ///
    /// 元数据钩子会在文件存储时提取和验证元数据。
    /// 可以注册多个元数据钩子，提取的元数据会被合并。
    pub fn register_metadata_hook(&mut self, hook: Box<dyn MetadataHook>) {
        self.metadata_hooks.push(hook);
    }

    /// 注册一个清理钩子
    ///
    /// 清理钩子会在文件被清理删除时被调用。
    /// 可以注册多个清理钩子，每个都会在清理前后被调用。
    pub fn register_cleanup_hook(&mut self, hook: Box<dyn CleanupHook>) {
        self.cleanup_hooks.push(hook);
    }

    // -------------------------------------------------------------------------
    // 执行方法（由 FileManager 内部调用）
    // -------------------------------------------------------------------------

    /// 执行所有存储前钩子
    ///
    /// 依次执行每个已注册的 `before_store` 钩子。
    /// 如果任何钩子返回 `Stop` 或 `Error`，立即停止。
    /// 如果任何钩子返回 `Modify`，后续钩子会收到修改后的数据。
    ///
    /// # 参数
    /// - `data`: 原始数据
    /// - `metadata`: 文件元数据
    ///
    /// # 返回
    /// - `Ok((data, should_continue))`: 继续执行，数据可能是修改后的
    /// - `Err(e)`: 被某个钩子阻止
    pub(crate) async fn run_before_store(
        &self,
        data: &[u8],
        metadata: &FileMetadata,
    ) -> Result<(Vec<u8>, bool)> {
        let mut current_data = data.to_vec();
        let mut should_continue = true;

        for hook in &self.storage_hooks {
            match hook.before_store(&current_data, metadata).await? {
                HookAction::Continue => {
                    // 继续，不修改数据
                }
                HookAction::Modify(new_data) => {
                    // 钩子返回了修改后的数据
                    current_data = new_data;
                }
                HookAction::Stop => {
                    // 钩子请求停止
                    should_continue = false;
                    break;
                }
                HookAction::Error(msg) => {
                    return Err(crate::FileError::Storage(msg));
                }
            }
        }

        Ok((current_data, should_continue))
    }

    /// 执行所有存储后钩子
    ///
    /// 依次执行每个已注册的 `after_store` 钩子。
    /// 错误会被记录但不会影响已完成的存储操作。
    pub(crate) async fn run_after_store(&self, handle: &FileHandle) {
        for hook in &self.storage_hooks {
            if let Err(e) = hook.after_store(handle).await {
                tracing::warn!("Storage hook after_store error: {}", e);
            }
        }
    }

    /// 执行所有读取前钩子
    ///
    /// 依次执行每个已注册的 `before_read` 钩子。
    /// 如果任何钩子返回 `Stop` 或 `Error`，立即停止。
    pub(crate) async fn run_before_read(&self, id: &str, requester: Option<&str>) -> Result<bool> {
        for hook in &self.read_hooks {
            match hook.before_read(id, requester).await? {
                HookAction::Continue => {}
                HookAction::Stop => return Ok(false),
                HookAction::Error(msg) => {
                    return Err(crate::FileError::Storage(msg));
                }
                HookAction::Modify(_) => {
                    // before_read 不应该返回 Modify，这是误用
                    tracing::warn!(
                        "Read hook returned Modify, which is not supported in before_read"
                    );
                }
            }
        }
        Ok(true)
    }

    /// 执行所有读取后钩子
    ///
    /// 依次执行每个已注册的 `after_read` 钩子。
    /// 前一个钩子的输出会传给下一个钩子作为输入。
    ///
    /// # 返回
    /// 最终的数据（可能经过多次转换）
    pub(crate) async fn run_after_read(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut current_data = data.to_vec();

        for hook in &self.read_hooks {
            match hook.after_read(&current_data).await? {
                HookAction::Continue => {
                    // 不修改，继续
                }
                HookAction::Modify(new_data) => {
                    current_data = new_data;
                }
                HookAction::Stop | HookAction::Error(_) => {
                    // after_read 不应该返回 Stop 或 Error
                    tracing::warn!("Read hook returned Stop/Error in after_read, which is ignored");
                }
            }
        }

        Ok(current_data)
    }

    /// 执行元数据提取钩子
    ///
    /// 依次执行每个已注册的 `extract_metadata` 钩子，
    /// 提取的元数据会被合并到一个 JSON 对象中。
    #[allow(dead_code)]
    pub(crate) async fn run_extract_metadata(
        &self,
        data: &[u8],
        base: &FileMetadata,
    ) -> Result<serde_json::Value> {
        let mut result = serde_json::json!({});

        for hook in &self.metadata_hooks {
            let extra = hook.extract_metadata(data, base).await?;
            // 合并元数据（简单的浅合并）
            if let serde_json::Value::Object(mut map) = extra {
                if let serde_json::Value::Object(base_map) = serde_json::to_value(&result)? {
                    map.extend(base_map);
                    result = serde_json::Value::Object(map);
                }
            }
        }

        Ok(result)
    }

    /// 执行元数据验证钩子
    ///
    /// 依次执行每个已注册的 `validate_metadata` 钩子。
    /// 如果任何钩子返回错误，立即停止并返回错误。
    pub(crate) async fn run_validate_metadata(&self, metadata: &FileMetadata) -> Result<()> {
        for hook in &self.metadata_hooks {
            hook.validate_metadata(metadata).await?;
        }
        Ok(())
    }

    /// 执行清理前检查钩子
    ///
    /// 依次执行每个已注册的 `should_cleanup` 钩子。
    /// 如果任何钩子返回 `false` 或错误，立即返回。
    pub(crate) async fn run_should_cleanup(
        &self,
        entry: &crate::handle::FileIndexEntry,
    ) -> Result<bool> {
        for hook in &self.cleanup_hooks {
            if !hook.should_cleanup(entry).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// 执行清理后钩子
    ///
    /// 依次执行每个已注册的 `after_cleanup` 钩子。
    /// 错误会被记录但不会回滚删除操作。
    pub(crate) async fn run_after_cleanup(&self, entry: &crate::handle::FileIndexEntry) {
        for hook in &self.cleanup_hooks {
            if let Err(e) = hook.after_cleanup(entry).await {
                tracing::warn!("Cleanup hook after_cleanup error: {}", e);
            }
        }
    }

    /// 获取已注册钩子的数量统计
    ///
    /// 用于调试和监控。
    pub fn hook_counts(&self) -> HookCounts {
        HookCounts {
            storage: self.storage_hooks.len(),
            read: self.read_hooks.len(),
            metadata: self.metadata_hooks.len(),
            cleanup: self.cleanup_hooks.len(),
        }
    }
}

/// 钩子数量统计
#[derive(Debug, Clone, Default)]
pub struct HookCounts {
    /// 存储钩子数量
    pub storage: usize,
    /// 读取钩子数量
    pub read: usize,
    /// 元数据钩子数量
    pub metadata: usize,
    /// 清理钩子数量
    pub cleanup: usize,
}

// ============================================================================
// 预置钩子实现 - Built-in Hooks
// ============================================================================

// 预置钩子放在这里，方便用户直接使用而不需要自己实现

/// 日志钩子 - 记录所有存储和读取操作
///
/// # 用法
/// ```rust,ignore
/// let mut registry = HookRegistry::new();
/// registry.register_storage_hook(Box::new(LoggingStorageHook));
/// registry.register_read_hook(Box::new(LoggingReadHook));
/// ```
pub struct LoggingStorageHook;

#[async_trait]
impl StorageHook for LoggingStorageHook {
    async fn before_store(&self, data: &[u8], metadata: &FileMetadata) -> Result<HookAction> {
        tracing::info!(
            "Storage: storing file '{}' ({} bytes)",
            metadata.name,
            data.len()
        );
        Ok(HookAction::Continue)
    }

    async fn after_store(&self, handle: &FileHandle) -> Result<()> {
        tracing::info!("Storage: file stored with ID '{}'", handle.id);
        Ok(())
    }
}

/// 日志读取钩子
pub struct LoggingReadHook;

#[async_trait]
impl ReadHook for LoggingReadHook {
    async fn before_read(&self, id: &str, requester: Option<&str>) -> Result<HookAction> {
        tracing::debug!("Read: reading file '{}' (requested by {:?})", id, requester);
        Ok(HookAction::Continue)
    }

    async fn after_read(&self, data: &[u8]) -> Result<HookAction> {
        tracing::debug!("Read: read {} bytes", data.len());
        Ok(HookAction::Modify(data.to_vec()))
    }
}

/// 清理日志钩子 - 记录清理操作
pub struct LoggingCleanupHook;

#[async_trait]
impl CleanupHook for LoggingCleanupHook {
    async fn should_cleanup(&self, entry: &crate::handle::FileIndexEntry) -> Result<bool> {
        tracing::debug!(
            "Cleanup: checking if '{}' (ref_count={}) should be cleaned up",
            entry.id,
            entry.ref_count
        );
        Ok(true)
    }

    async fn after_cleanup(&self, entry: &crate::handle::FileIndexEntry) -> Result<()> {
        tracing::info!("Cleanup: file '{}' has been deleted", entry.id);
        Ok(())
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // 测试辅助：创建一个简单的存储钩子
    struct TestStorageHook {
        before_called: std::sync::Arc<std::sync::atomic::AtomicBool>,
        after_called: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl TestStorageHook {
        fn new() -> (
            Self,
            std::sync::Arc<std::sync::atomic::AtomicBool>,
            std::sync::Arc<std::sync::atomic::AtomicBool>,
        ) {
            let before_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let after_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let bc = before_called.clone();
            let ac = after_called.clone();
            (
                Self {
                    before_called,
                    after_called,
                },
                bc,
                ac,
            )
        }
    }

    #[async_trait]
    impl StorageHook for TestStorageHook {
        async fn before_store(&self, _data: &[u8], metadata: &FileMetadata) -> Result<HookAction> {
            self.before_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
            tracing::debug!("Test hook: before_store for '{}'", metadata.name);
            Ok(HookAction::Continue)
        }

        async fn after_store(&self, handle: &FileHandle) -> Result<()> {
            self.after_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
            tracing::debug!("Test hook: after_store for '{}'", handle.id);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_hook_registry_register_and_count() {
        let mut registry = HookRegistry::new();

        assert_eq!(registry.hook_counts().storage, 0);

        registry.register_storage_hook(Box::new(TestStorageHook::new().0));
        assert_eq!(registry.hook_counts().storage, 1);

        registry.register_read_hook(Box::new(TestReadHook));
        assert_eq!(registry.hook_counts().read, 1);
    }

    #[tokio::test]
    async fn test_hook_action_modify_carries_data() {
        let action = HookAction::Modify(vec![1, 2, 3]);

        assert!(action.should_continue());
        assert!(action.has_modified_data());
        assert_eq!(action.get_modified_data(), Some(vec![1, 2, 3]));
    }

    #[tokio::test]
    async fn test_hook_action_stop_does_not_continue() {
        let action = HookAction::Stop;
        assert!(!action.should_continue());
        assert!(!action.has_modified_data());
    }

    // 测试辅助：简单的读取钩子
    struct TestReadHook;

    #[async_trait]
    impl ReadHook for TestReadHook {
        async fn before_read(&self, _id: &str, _requester: Option<&str>) -> Result<HookAction> {
            Ok(HookAction::Continue)
        }

        async fn after_read(&self, data: &[u8]) -> Result<HookAction> {
            Ok(HookAction::Modify(data.to_vec()))
        }
    }
}
