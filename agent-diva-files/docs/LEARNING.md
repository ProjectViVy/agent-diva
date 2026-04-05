# Agent Diva Files 模块学习课程

本课程详细介绍 `agent-diva-files` Rust 模块的设计理念、核心概念与使用方法。通过本课程，你将理解如何实现一个高效、可靠的内容寻址存储系统。

---

## 目录

1. [概览](#概览)
2. [核心概念](#核心概念)
3. [模块架构](#模块架构)
4. [Hook 系统](#hook-系统)
5. [软删除机制](#软删除机制)
6. [Channel 文件管理](#channel-文件管理)
7. [使用示例](#使用示例)
8. [进一步阅读](#进一步阅读)

---

## 概览

`agent-diva-files` 是一个用 Rust 编写的**内容寻址存储模块**，为 Agent Diva 框架提供文件管理能力。

### 设计目标

- **去重存储**：相同内容的文件只存储一份，节省空间
- **引用计数**：安全管理文件生命周期，无被引用时才能删除
- **可扩展性**：通过 Hook 系统支持压缩、加密、权限检查等自定义行为
- **软删除**：Steam 风格的"回收站"机制，支持恢复和自动清理

### 关键特性

| 特性 | 说明 |
|------|------|
| SHA256 内容寻址 | 文件按内容 hash 存储，相同内容自动去重 |
| 引用计数 | 多引用共享，ref_count=0 时才真正删除 |
| Hook 钩子系统 | 存储/读取/清理各阶段可注入自定义逻辑 |
| 软删除 | deleted_at + deleted_by 标记，支持恢复 |
| Channel 隔离 | 逻辑上隔离文件，可属于多个 Channel |

---

## 核心概念

### 1. 内容寻址存储 (Content-Addressed Storage)

传统的文件存储按文件名或 ID 寻址，而内容寻址存储按**内容 hash** 寻址。

```
传统方式: filename → disk location
内容寻址: content → SHA256 hash → disk location
```

**为什么用 SHA256？**
- 抗碰撞：找到两个相同 hash 的不同内容在计算上不可行
- 确定性：相同内容始终产生相同的 hash
- 用途广泛：区块链、Git、IPFS 都使用类似机制

**在 agent-diva-files 中：**
```rust
// 文件 ID 即为 SHA256 hash
let file_id = format!("sha256:{}", sha256_hash);

// 存储路径由 hash 决定
let path = hash_to_path(&hash); // sha256:abc123 → ab/c123
```

### 2. 引用计数 (Reference Counting)

一个物理文件可以被多个逻辑引用共享。每增加一个引用，ref_count +1；每释放一个引用，ref_count -1。

```
场景：同一张图片被发送给 3 个用户

消息1 → FileHandle(id=abc123, ref_count=1)
消息2 → FileHandle(id=abc123, ref_count=2)  ← 共用同一物理文件
消息3 → FileHandle(id=abc123, ref_count=3)

当消息1的引用被释放：
    ref_count = 2  (文件保留)
当消息2的引用也被释放：
    ref_count = 1  (文件保留)
当消息3的引用也被释放：
    ref_count = 0  → 文件可被清理
```

**这解决了什么问题？**
- 避免重复存储：相同内容只存一份
- 安全删除：只有 ref_count=0 时才真正删除
- 原子性：并发场景下计数器的正确管理

### 3. SQLite 持久化索引

文件元数据存储在 SQLite 数据库中（`index.db`），而非内存中的 HashMap。

**为什么用 SQLite？**
- ACID 事务：崩溃恢复不丢数据
- SQL 查询：支持复杂的筛选和聚合
- 持久化：程序重启后索引不丢失
- 并发安全：多个连接可以同时读写

**数据库表设计：**
```sql
CREATE TABLE files (
    id TEXT PRIMARY KEY,           -- SHA256 hash
    path TEXT NOT NULL,            -- 物理存储路径
    size INTEGER NOT NULL,        -- 文件大小
    ref_count INTEGER NOT NULL,    -- 引用计数
    created_at TEXT NOT NULL,     -- 创建时间
    last_accessed_at TEXT,        -- 最后访问时间
    deleted_at TEXT,              -- 软删除时间 (新增)
    deleted_by TEXT,              -- 删除者标识 (新增)
    metadata_json TEXT NOT NULL   -- 序列化元数据
);
```

---

## 模块架构

```
agent-diva-files/
├── src/
│   ├── lib.rs          # 公共 API 导出
│   ├── config.rs       # 配置管理
│   ├── storage.rs      # 文件 I/O 和 SHA256 计算
│   ├── backend.rs      # 存储后端 trait + 本地实现
│   ├── handle.rs       # FileHandle 和元数据类型
│   ├── index.rs        # SQLite 索引实现
│   ├── manager.rs      # FileManager 主接口
│   ├── hooks.rs        # Hook 钩子系统
│   └── channel.rs      # Channel 文件管理
└── docs/
    └── LEARNING.md     # 本教程
```

### 核心类型

| 类型 | 所在文件 | 职责 |
|------|----------|------|
| `FileManager` | manager.rs | 主入口，协调所有操作 |
| `FileHandle` | handle.rs | 文件引用句柄 |
| `FileMetadata` | handle.rs | 文件元数据 |
| `SqliteIndex` | index.rs | SQLite 持久化索引 |
| `LocalStorageBackend` | backend.rs | 本地文件系统后端 |
| `HookRegistry` | hooks.rs | Hook 注册与执行 |

### 数据流

```
用户代码
    ↓
FileManager::store(data, metadata)
    ↓
1. 计算 SHA256 hash
    ↓
2. HookRegistry::before_store (可修改数据)
    ↓
3. 检查是否已存在 (去重)
    ↓
    ├─ 已存在 → ref_count++, 返回现有 handle
    └─ 不存在 → 写入 storage, 创建索引, 返回新 handle
    ↓
4. HookRegistry::after_store (记录日志等)
    ↓
FileHandle { id, path, metadata }
```

---

## Hook 系统

Hook 系统允许在文件操作的各个阶段注入自定义逻辑，实现**横切关注点分离**。

### Hook 类型

| Hook | 触发时机 | 用途 |
|------|----------|------|
| `StorageHook` | 存储前/后 | 压缩、加密、病毒扫描 |
| `ReadHook` | 读取前/后 | 权限检查、日志记录 |
| `MetadataHook` | 元数据提取/验证 | 自定义元数据、格式检测 |
| `CleanupHook` | 清理判断/执行后 | 彻底删除关联数据、通知 |

### HookAction 返回值

每个 Hook 方法返回 `HookAction`，决定后续行为：

```rust
pub enum HookAction {
    /// 继续执行，不做任何修改
    Continue,
    /// 修改数据 (携带新数据继续)
    Modify(Vec<u8>),
    /// 停止执行 (操作被取消)
    Stop,
    /// 出错 (携带错误终止)
    Error(String),
}
```

### 示例：日志 Hook

```rust
use agent_diva_files::hooks::{HookAction, StorageHook, HookRegistry};
use agent_diva_files::handle::FileMetadata;
use async_trait::async_trait;

struct LoggingStorageHook;

#[async_trait]
impl StorageHook for LoggingStorageHook {
    async fn before_store(&self, data: &[u8], metadata: &FileMetadata) -> Result<HookAction> {
        tracing::info!("Storing file: {} ({} bytes)", metadata.name, data.len());
        Ok(HookAction::Continue)
    }

    async fn after_store(&self, handle: &FileHandle) -> Result<()> {
        tracing::info!("File stored: {}", handle.id);
        Ok(())
    }
}

// 注册 Hook
let mut registry = HookRegistry::new();
registry.register_storage_hook(Box::new(LoggingStorageHook));
```

### 内置 Hook 实现

`hooks.rs` 提供了内置的日志 Hook：

- `LoggingStorageHook` - 记录存储操作
- `LoggingReadHook` - 记录读取操作
- `LoggingCleanupHook` - 记录清理操作

---

## 软删除机制

软删除（Soft Delete）实现 **Steam 风格回收站**：文件被"删除"后不立即物理删除，而是标记删除时间和删除者，可在保留期内恢复或自动彻底清理。

### 核心字段

```rust
// 在 FileIndexEntry 中
deleted_at: Option<DateTime<Utc>>,  // 软删除时间
deleted_by: Option<String>,          // 删除者标识
```

### API 方法

| 方法 | 说明 |
|------|------|
| `soft_delete(id, deleted_by)` | 标记文件为已删除 |
| `restore(id)` | 恢复已删除文件 |
| `list_deleted()` | 列出所有已删除文件 |
| `hard_delete(id)` | 物理删除（不经过回收期）|
| `purge_expired(retention_days)` | 清理超过保留期的文件 |

### 工作流程

```
用户删除文件
    ↓
soft_delete(id, "user123")
    ↓
index.soft_delete() 更新数据库
    ↓
deleted_at = now()
deleted_by = "user123"
    ↓
文件不在正常列表中出现，但仍占用空间

7 天后（可配置）
    ↓
purge_expired(7) 被调用
    ↓
查找 deleted_at < now() - 7天 的文件
    ↓
物理删除文件 + 索引条目
```

### 与 Cleanup 的区别

| 特性 | `cleanup()` | `soft_delete()` + `purge_expired()` |
|------|-------------|-------------------------------------|
| 触发条件 | ref_count=0 + 超时 | 显式删除调用 |
| 删除方式 | 直接物理删除 | 标记 + 延迟删除 |
| 恢复能力 | 不可恢复 | 可在保留期内恢复 |
| 适用场景 | 临时缓存、过期文件 | 用户主动删除的文件 |

---

## Channel 文件管理

Channel 系统实现**逻辑隔离的文件管理**：物理存储是共享的（复用 SHA256 去重），但文件可以属于多个逻辑频道。

### 核心思想

```
全局文件存储:
  sha256:abc123 → /data/ab/c123 (物理存储)
                     ↑
                     │ 引用计数 ref_count = 3
                     │
频道关联:
  channel:telegram:chat_1 → sha256:abc123
  channel:discord:server_2 → sha256:abc123
```

同一文件在物理上只存储一份，但在逻辑上可属于多个 Channel。

### 数据库设计

```sql
-- Channel 关联表
CREATE TABLE channel_files (
    id INTEGER PRIMARY KEY,
    channel_id TEXT NOT NULL,      -- 频道标识符
    file_id TEXT NOT NULL,          -- 关联的文件ID
    uploaded_by TEXT,               -- 上传者
    uploaded_at TEXT NOT NULL,      -- 上传时间
    message_id TEXT,                -- 关联的消息ID
    UNIQUE(channel_id, file_id)
);

-- Channel 统计表
CREATE TABLE channel_stats (
    channel_id TEXT PRIMARY KEY,
    total_files INTEGER DEFAULT 0,
    total_size INTEGER DEFAULT 0,
    last_updated TEXT
);
```

### API 方法

| 方法 | 说明 |
|------|------|
| `upload_to_channel(channel_id, data, metadata, ...)` | 上传文件到 Channel |
| `add_file_to_channel(channel_id, file_id, ...)` | 已有文件加入 Channel |
| `list_channel_files(channel_id)` | 列出 Channel 中文件 |
| `remove_from_channel(channel_id, file_id)` | 从 Channel 移除 |
| `delete_channel(channel_id, cleanup)` | 删除整个 Channel |
| `list_file_channels(file_id)` | 查看文件属于哪些 Channel |
| `channel_stats(channel_id)` | 获取 Channel 统计 |

### 使用场景

**场景 1: Telegram 群组文件管理**

```rust
// 上传文件到 Telegram 频道
let handle = channel_manager
    .upload_to_channel(
        "telegram:chat_123",
        data,
        metadata,
        Some("user1"),
        Some("msg_456"),
    )
    .await?;

// 列出该群组的所有文件
let files = channel_manager.list_channel_files("telegram:chat_123").await?;
```

**场景 2: Discord 服务器共享**

```rust
// 同一文件分享到多个 Discord 服务器
let handle = channel_manager
    .upload_to_channel("discord:server_1", data, metadata, None, None)
    .await?;

channel_manager
    .add_file_to_channel("discord:server_2", &handle.id, Some("admin"), None)
    .await?;
```

---

## 使用示例

### 基础存储操作

```rust
use agent_diva_files::{FileManager, FileConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建配置和管理器
    let config = FileConfig::with_path(PathBuf::from("./data"));
    let manager = FileManager::new(config).await?;

    // 存储文件
    let data = b"Hello, World!";
    let metadata = FileMetadata {
        name: "hello.txt".to_string(),
        size: data.len() as u64,
        mime_type: Some("text/plain".to_string()),
        source: Some("cli".to_string()),
        created_at: chrono::Utc::now(),
        last_accessed_at: None,
        preview: None,
    };

    let handle = manager.store(data, metadata).await?;
    println!("Stored file with ID: {}", handle.id);

    // 读取文件
    let content = manager.read(&handle).await?;
    println!("File content: {}", String::from_utf8_lossy(&content));

    // 释放引用
    manager.release(&handle).await?;
    println!("File reference released");

    Ok(())
}
```

### 带 Hook 的存储

```rust
use agent_diva_files::{FileManager, HookRegistry};
use agent_diva_files::hooks::{StorageHook, HookAction};
use async_trait::async_trait;

// 自定义压缩 Hook
struct CompressionHook;

#[async_trait]
impl StorageHook for CompressionHook {
    async fn before_store(&self, data: &[u8], _metadata: &FileMetadata) -> Result<HookAction> {
        let compressed = compress(data);
        Ok(HookAction::Modify(compressed))
    }
}

let hooks = HookRegistry::new();
hooks.register_storage_hook(Box::new(CompressionHook));

let manager = FileManager::new_with_hooks(config, hooks).await?;
```

### 软删除与恢复

```rust
// 软删除
manager.soft_delete(&handle.id, Some("admin_user")).await?;

// 列出已删除文件
let deleted = manager.list_deleted().await?;
for file in deleted {
    println!("Deleted: {} at {}", file.id, file.deleted_at.unwrap());
}

// 恢复
manager.restore(&handle.id).await?;

// 强制删除 (绕过保留期)
manager.hard_delete(&handle.id).await?;
```

### Channel 文件操作

```rust
use agent_diva_files::channel::ChannelManager;

let channel_manager = ChannelManager::new(
    Arc::new(manager),
    PathBuf::from("./channels.db"),
).await?;

// 上传到 Channel
let handle = channel_manager
    .upload_to_channel(
        "telegram:chat_123",
        data,
        metadata,
        Some("user1"),
        Some("msg_456"),
    )
    .await?;

// 列出 Channel 文件
let files = channel_manager.list_channel_files("telegram:chat_123").await?;

// 获取 Channel 统计
let stats = channel_manager.channel_stats("telegram:chat_123").await?;
println!("Channel has {} files, total size: {}", stats.total_files, stats.total_size);
```

---

## 进一步阅读

### 源码文件

| 文件 | 内容 |
|------|------|
| `src/lib.rs` | 公共 API 导出 |
| `src/manager.rs` | FileManager 主逻辑 |
| `src/hooks.rs` | Hook 系统详细实现 |
| `src/index.rs` | SQLite 索引实现 |
| `src/channel.rs` | Channel 管理实现 |

### 调试与开发

- [调试指南](./debugging.md) - 一步步教你如何调试 agent-diva-files

### 设计文档

- [Phase 1 进度](../memory/agent-diva-files-phase1.md) - Phase 1 完成状态
- [Phase 3 设计](./agent-diva-files-phase3.md) - Hook 系统与软删除设计

### 相关技术

- **Rust 异步编程**: `tokio` 异步运行时
- **SQLite + sqlx**: 异步数据库访问
- **内容寻址**: [IPFS](https://ipfs.io/) 使用的同类型存储
- **引用计数**: Git 的对象引用机制

---

## 常见问题

**Q: 文件被删除后，物理存储空间何时释放？**

A: 有两种机制：
1. `cleanup()` - 当 ref_count 降至 0 且超过 max_age_days 时清理
2. `purge_expired()` - 软删除超过 retention_days 后彻底删除

**Q: 如何实现文件的加密存储？**

A: 通过 StorageHook：
```rust
impl StorageHook for EncryptionHook {
    async fn before_store(&self, data: &[u8], _: &FileMetadata) -> Result<HookAction> {
        let encrypted = encrypt(data, &self.key);
        Ok(HookAction::Modify(encrypted))
    }
}
```

**Q: Channel 和普通文件引用的区别是什么？**

A: Channel 只是在逻辑上标记文件归属，不影响 ref_count。同一文件可以属于多个 Channel，ref_count 由 FileManager 的 store/release 维护。

**Q: 数据库迁移如何处理？**

A: `SqliteIndex::migrate_from_jsonl()` 支持从旧版 JSONL 格式迁移到 SQLite。

---

*本课程最后更新于 2026-03-30*
