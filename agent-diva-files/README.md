# agent-diva-files

> 内容寻址存储模块 — 为 Agent Diva 提供文件管理能力

## 特性

- **内容寻址**：SHA256 哈希去重，相同内容只存一份
- **引用计数**：安全生命周期管理，ref_count=0 才真正删除
- **Hook 系统**：存储/读取/清理各阶段可注入自定义逻辑
- **软删除**：Steam 风格回收站，支持恢复和自动清理
- **Channel 隔离**：逻辑隔离文件，可属于多个 Channel

## 快速开始

### 环境要求

- Rust 1.75+
- tokio 异步运行时

### 构建

```bash
# Debug 构建
cargo build

# Release 构建
cargo build --release
```

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行指定测试
cargo test test_store_and_get

# 带日志输出运行
cargo test -- --nocapture
```

### 代码检查

```bash
# 格式化检查
cargo fmt -- --check

# 格式化修复
cargo fmt

# Clippy 检查
cargo clippy -- -D warnings
```

### 完整 CI 检查

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## 使用示例

```rust
use agent_diva_files::{FileManager, FileConfig, FileMetadata};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建管理器
    let config = FileConfig::with_path(PathBuf::from("./data"));
    let manager = FileManager::new(config).await?;

    // 存储文件
    let data = b"Hello, World!";
    let metadata = FileMetadata {
        name: "hello.txt".to_string(),
        size: data.len() as u64,
        mime_type: Some("text/plain".to_string()),
        source: Some("example".to_string()),
        created_at: chrono::Utc::now(),
        last_accessed_at: None,
        preview: None,
    };

    let handle = manager.store(data, metadata).await?;
    println!("Stored: {}", handle.id);

    // 读取文件
    let content = manager.read(&handle).await?;
    println!("Content: {}", String::from_utf8_lossy(&content));

    // 释放引用
    manager.release(&handle).await?;

    Ok(())
}
```

## 模块结构

```
src/
├── lib.rs          # 公共 API 导出
├── config.rs       # 配置管理
├── storage.rs      # 文件 I/O 和 SHA256 计算
├── backend.rs      # 存储后端 trait
├── handle.rs       # FileHandle 和元数据
├── index.rs       # SQLite 索引
├── manager.rs     # FileManager 主接口
├── hooks.rs       # Hook 钩子系统
└── channel.rs     # Channel 文件管理
```

## 学习资源

- [详细教程](./docs/LEARNING.md) — 深入理解设计原理
- [验收标准](./docs/acceptance.md) — 功能测试清单
