---
module: agent-diva-migration
kind: rust-binary
msrv: "1.80.0"
workspace: agent-diva
---

## 模块职责

- 独立工具 `agent-diva-migrate`：从旧版 **Python** agent-diva 迁移配置、会话、记忆等到 Rust 版目录布局。
- 子模块：`config_migration`、`session_migration`、`memory_migration`；支持 `--dry-run`、分步跳过（`skip_*`）与 `-y` 自动确认。

## 依赖与边界

- **内部**：仅直接依赖 `agent-diva-core`（路径依赖，注意未锁 `version`，与兄弟 crate 保持一致即可）。
- **Workspace**：`tokio`、`serde`、`serde_json`、`anyhow`、`tracing`、`tracing-subscriber`、`clap`、`console`、`dialoguer`、`dirs`；`tempfile` 用于测试/临时场景。
- **边界**：不写网关、不提供 HTTP；仅文件/配置变换与路径约定。迁移规则变更需与 `agent-diva-core` 的 `Config` 结构兼容。

## 关键入口

- 二进制：`src/main.rs`（`#[tokio::main] async fn main`），`[[bin]] name = "agent-diva-migrate"`。

## 实现约定

- **MSRV**：`rust-version = "1.80.0"`。
- CLI 使用 `clap` derive；交互确认使用 `dialoguer`，输出着色使用 `console::style`。
- 默认源/目标目录逻辑与 `get_default_agent_diva_dir` 保持一致（通常为 `~/.agent-diva` 类路径）。

## 测试与检查

- `cargo test -p agent-diva-migration`；`config_migration.rs`、`session_migration.rs`、`memory_migration.rs` 含单元测试块。
- 手工验收：对副本目录先 `--dry-run`，再小步真实迁移。

## 切勿遗漏

- Python 侧路径或文件名约定变化时，必须更新对应 migrator 并补测试。
- 包版本 `0.4.0` 与 workspace 根 `0.4.1` 可能不一致，发版前统一核对。
