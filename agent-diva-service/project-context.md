---
module: agent-diva-service
kind: rust-binary-windows-only
msrv: "1.80.0"
workspace: agent-diva
---

## 模块职责

- Windows 服务封装：以服务名 `AgentDivaGateway` 注册，生命周期内拉起同目录下的 `agent-diva.exe` 执行网关（等价于 CLI `gateway run` 场景），处理 Stop/Shutdown。
- 提供 `--console` 便于本机调试（非服务模式子进程等待）。

## 依赖与边界

- **依赖极精简**：`anyhow`、`tracing`、`tracing-subscriber`、`clap`；Windows 下 `windows-service`（workspace）。
- **非 Windows**：`main` 直接报错退出，禁止假装可用。
- **边界**：不包含业务配置解析实现；仅转发 `config_dir` 给子进程。网关逻辑仍在 `agent-diva` CLI + `agent-diva-manager` 运行时。

## 关键入口

- 唯一二进制：`src/main.rs` → `[[bin]] agent-diva-service`。
- Windows 实现集中在 `#[cfg(windows)] mod windows_impl`。

## 实现约定

- **MSRV**：本包 `Cargo.toml` 未写 `rust-version` 时，仍以 workspace `rust-version = "1.80.0"` 为底线，本地与 CI 使用 ≥1.80 工具链。
- 子进程路径：`sibling_cli_path` 解析为与当前 exe 同目录的 `agent-diva.exe`，发布安装包时必须保证二者同目录。
- 日志：`tracing_subscriber` + `RUST_LOG` 风格 env filter，默认 `info`。

## 测试与检查

- 本 crate 几乎无单元测试；验证以 Windows 上 `--console` 与真实服务安装为主。
- `cargo check -p agent-diva-service`；在非 Windows 上应能通过编译并仅在运行时拒绝执行。

## 切勿遗漏

- 修改服务名、子进程参数或与 CLI 的约定时，同步安装脚本/文档。
- 勿在此 crate 引入重型 workspace 依赖，以免服务二进制膨胀。
