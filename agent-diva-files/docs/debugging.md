# Build 与 Run 指南

本文档介绍如何构建和运行 agent-diva-files。

## 快速开始

### 1. 环境要求

- **Rust 1.80+** ([安装](https://rustup.rs/))
- **Cargo** (随 Rust 自动安装)

验证安装：
```bash
rustc --version
cargo --version
```

### 2. 构建项目

```bash
# 克隆项目
git clone https://github.com/ProjectViVy/agent-diva.git
cd agent-diva

# Debug 构建 (快速编译)
cargo build -p agent-diva-files

# Release 构建 (优化后，发布用)
cargo build -p agent-diva-files --release

# 构建所有 crates
cargo build --all
```

### 3. 运行测试

```bash
# 运行所有测试
cargo test -p agent-diva-files

# 运行特定测试
cargo test -p agent-diva-files test_store_and_get

# 带日志输出运行
cargo test -p agent-diva-files -- --nocapture

# 运行测试并显示 println!
cargo test -p agent-diva-files -- --show-output
```

### 4. 运行示例代码

创建 `examples/demo.rs`：

```rust
use agent_diva_files::{FileManager, FileConfig, FileMetadata};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建配置和管理器
    let config = FileConfig::with_path(PathBuf::from("./test_data"));
    let manager = FileManager::new(config).await?;

    // 存储文件
    let data = b"Hello, World!";
    let metadata = FileMetadata {
        name: "hello.txt".to_string(),
        size: data.len() as u64,
        mime_type: Some("text/plain".to_string()),
        source: Some("demo".to_string()),
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
    println!("Done!");

    Ok(())
}
```

运行示例：
```bash
cargo run --example demo -p agent-diva-files
```

### 5. 使用 CLI 工具

```bash
# 查看帮助
cargo run -p agent-diva-cli -- --help

# 运行 CLI
cargo run -p agent-diva-cli -- status
```

## 项目结构

```
agent-diva/
├── agent-diva-files/           # 文件存储核心模块
│   ├── src/
│   │   ├── lib.rs             # 公共 API
│   │   ├── manager.rs          # FileManager 主入口
│   │   ├── storage.rs          # 文件 I/O
│   │   ├── index.rs            # SQLite 索引
│   │   ├── hooks.rs            # Hook 系统
│   │   └── channel.rs          # Channel 管理
│   └── docs/
│       ├── README.md           # 快速上手
│       ├── LEARNING.md          # 详细教程
│       └── debugging.md         # 调试指南
├── agent-diva-core/           # 核心类型
├── agent-diva-agent/           # Agent 实现
└── ...
```

## 常用命令

```bash
# 代码检查
cargo check -p agent-diva-files          # 快速检查
cargo clippy -p agent-diva-files -D warnings   # Lint

# 格式化
cargo fmt -p agent-diva-files            # 格式化代码
cargo fmt -p agent-diva-files -- --check # 检查格式

# 清理构建
cargo clean -p agent-diva-files          # 清理单个 crate
cargo clean                              # 清理整个项目

# 依赖分析
cargo tree -p agent-diva-files           # 查看依赖树
cargo tree -p agent-diva-files --invert  # 反向依赖（谁依赖它）
```

## 代码检查 (CI 流程)

```bash
# 完整检查
cargo fmt
cargo clippy -D warnings
cargo test -p agent-diva-files
```

## 创建新功能流程

1. **创建分支**
   ```bash
   git checkout -b feature/my-feature
   ```

2. **编写代码和测试**
   ```bash
   cargo test -p agent-diva-files -- test_my_feature
   ```

3. **调试**
   ```bash
   cargo test -p agent-diva-files test_my_feature -- --nocapture
   ```

4. **提交**
   ```bash
   git add .
   git commit -m "feat: add my feature"
   ```

---

# 调试指南

本文档介绍如何在 agent-diva-files 中进行调试。

## 环境准备

### 1. 安装 Rust 调试工具

```bash
# 安装 rust-analyzer (VS Code 扩展会自动提示)
# 安装 CodeLLDB (macOS/Linux) 或 C++ debugger (Windows)

# Windows: 通过 Visual Studio Installer 安装 "C++ 桌面开发"
# macOS: Xcode 自带 lldb
# Linux: sudo apt install lldb
```

### 2. 克隆项目

```bash
git clone https://github.com/ProjectViVy/agent-diva.git
cd agent-diva
```

### 3. 验证构建

```bash
cargo build -p agent-diva-files
```

## 使用 VS Code 调试

### 配置 launch.json

在 `.vscode/launch.json` 中添加配置：

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",           // Windows 用 "cppvsdbg"
            "request": "launch",
            "name": "调试 agent-diva-files 测试",
            "cargo": {
                "args": [
                    "test",
                    "-p", "agent-diva-files",
                    "--no-run"
                ],
                "filter": {
                    "name": "agent-diva-files"
                }
            },
            "cwd": "${workspaceFolder}"
        },
        {
            "type": "lldb",
            "request": "launch",
            "name": "调试单个测试",
            "cargo": {
                "args": [
                    "test",
                    "-p", "agent-diva-files",
                    "--no-run",
                    "--",
                    "test_store_and_get"    // 替换为实际测试名
                ],
                "filter": {
                    "name": "agent-diva-files"
                }
            },
            "cwd": "${workspaceFolder}"
        }
    ]
}
```

### 断点调试步骤

1. **打开测试文件**：`agent-diva-files/src/manager.rs`

2. **设置断点**：点击行号左侧空白处
   - `store()` 方法入口
   - `read()` 方法入口
   - Hook 执行位置

3. **启动调试**：
   - 按 `F5` 或点击调试面板的绿色箭头
   - 选择 "调试单个测试"

4. **单步执行**：
   - `F10` - 单步跳过
   - `F11` - 单步进入
   - `Shift+F11` - 单步退出
   - `F5` - 继续运行到下一个断点

## 使用命令行调试

### 运行特定测试

```bash
# 运行单个测试并输出
cargo test -p agent-diva-files test_store_and_get -- --nocapture

# 运行测试并显示打印
cargo test -p agent-diva-files test_store_and_get -- --show-output
```

### 查看详细错误

```bash
# 详细错误输出
cargo test -p agent-diva-files -- --show-output 2>&1

# 测试失败时立即停止
cargo test -p agent-diva-files -- --fail-fast
```

### 检查内存问题

```bash
# macOS
cargo test -p agent-diva-files
leaks --atExit -- cargo test -p agent-diva-files

# Linux
valgrind --leak-check=full cargo test -p agent-diva-files
```

## 日志调试

### 启用 tracing 日志

```rust
// 在测试中使用 tracing
#[tokio::test]
async fn test_store_and_get() {
    // 初始化 subscriber
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // 你的测试代码
    let manager = FileManager::new(config).await.unwrap();
    // ...
}
```

### 使用环境变量

```bash
# 运行时启用日志
RUST_LOG=debug cargo test -p agent-diva-files

# 特定模块日志
RUST_LOG=agent_diva_files=debug cargo test -p agent-diva-files
RUST_LOG=agent_diva_files::manager=trace cargo test -p agent-diva-files
```

## 常见问题排查

### 1. 测试失败：文件未找到

```bash
# 检查测试数据目录
ls -la ./test_data/

# 手动清理测试数据
rm -rf ./test_data/
```

### 2. 并发测试失败

检查是否有共享状态冲突：

```bash
# 使用 thread-sanitizer
RUSTFLAGS="-- sanitizer=thread" cargo test -p agent-diva-files
```

### 3. 性能问题

```bash
# 安装火焰图工具
cargo install flamegraph

# 生成火焰图
cargo flamegraph -p agent-diva-files --test test_benchmark
```

### 4. 数据库问题

```bash
# 查看 SQLite 数据库内容
sqlite3 ./test_data/file_index.db ".schema"

# 查看所有记录
sqlite3 ./test_data/file_index.db "SELECT * FROM files;"
```

## 调试 Hook 系统

Hook 系统的调试重点：

1. **设置断点**在 `agent-diva-files/src/hooks.rs`
   - `HookRegistry::execute_pre_store()`
   - `HookRegistry::execute_post_store()`

2. **验证 Hook 执行顺序**
   ```rust
   // 添加调试日志
   println!("Hook 执行: {:?}", hook_name);
   ```

3. **测试自定义 Hook**
   ```rust
   struct DebugHook;
   impl StorageHook for DebugHook {
       fn pre_store(&self, ctx: &HookContext) -> HookResult {
           println!("pre_store called with {:?}", ctx);
           HookResult::Continue
       }
   }
   ```

## 调试 Channel 功能

1. **验证 Channel 隔离**
   ```bash
   cargo test -p agent-diva-files test_channel_isolation -- --nocapture
   ```

2. **检查 Channel 统计**
   ```rust
   let stats = manager.channel_stats("telegram").await?;
   println!("Channel 统计: {:?}", stats);
   ```

## 调试软删除

1. **测试删除/恢复流程**
   ```bash
   cargo test -p agent-diva-files test_soft_delete -- --nocapture
   ```

2. **验证保留期逻辑**
   ```bash
   # 设置短保留期进行测试
   let config = FileConfig {
       retention_days: 1,
       ..Default::default()
   };
   ```

## 获取帮助

- 查看 [LEARNING.md](./LEARNING.md) 深入理解设计原理
- 查看 [acceptance.md](./acceptance.md) 验收标准
- 运行完整测试套件：`cargo test -p agent-diva-files`
