# Phase 1：基础设施改造（MVP）

> 目标：暴露 agent-diva-manager 路由构建 API，创建 RAII ServerHandle，实现端口预绑定

---

## 1. 步骤概览

| 步骤 | 文件 | 操作 |
|------|------|------|
| 1.1 | `agent-diva-manager/src/server.rs` | 将 `build_app` 改为公开函数 |
| 1.2 | `agent-diva-manager/src/lib.rs` | 新增导出 `build_router` |
| 1.3 | `agent-diva-gui/src-tauri/Cargo.toml` | 新增依赖 agent-diva-manager |
| 1.4 | `agent-diva-gui/src-tauri/src/embedded_server.rs` | 新建 RAII ServerHandle |
| 1.5 | 测试验证 | 编译验证 + 基本启动测试 |

---

## 2. 详细步骤

### 2.1 暴露 agent-diva-manager 路由构建 API

**文件**：`agent-diva-manager/src/server.rs`

**当前代码（第47行）**：
```rust
fn build_app(state: AppState) -> Router {
    Router::new()
        .merge(runtime_routes())
        .merge(provider_routes())
        .merge(misc_routes())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
```

**改为**：
```rust
/// Build the axum Router with all API routes.
/// Public API for embedded server usage.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(runtime_routes())
        .merge(provider_routes())
        .merge(misc_routes())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
```

**改动说明**：
- 函数名从 `build_app` 改为 `build_router`（语义更清晰）
- 添加 `pub` 使其成为公开 API
- 添加文档注释

---

### 2.2 新增导出

**文件**：`agent-diva-manager/src/lib.rs`

**当前代码（第12行）**：
```rust
pub use server::run_server;
```

**新增导出**：
```rust
pub use server::{build_router, run_server};
```

---

### 2.3 新增 GUI 依赖

**文件**：`agent-diva-gui/src-tauri/Cargo.toml`

**在 `[dependencies]` 中新增**：
```toml
agent-diva-manager = { path = "../../agent-diva-manager" }
```

---

### 2.4 新建 embedded_server.rs

**文件**：`agent-diva-gui/src-tauri/src/embedded_server.rs`

**结构体定义**：

```rust
use agent_diva_manager::{build_router, AppState, GatewayRuntimeConfig};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::watch;
use tracing::{error, info};

/// Handle to the running embedded gateway.
/// Drop or call `shutdown()` to stop the server gracefully.
pub struct EmbeddedGatewayHandle {
    /// The port the server is listening on.
    pub port: u16,
    /// Send `true` to trigger graceful shutdown.
    shutdown_tx: watch::Sender<bool>,
    /// Join handle for the background server thread.
    server_thread: Option<std::thread::JoinHandle<()>>,
    /// Track whether shutdown has already been initiated.
    shutdown_initiated: Arc<AtomicBool>,
}
```

**核心方法**：

```rust
impl EmbeddedGatewayHandle {
    /// Signal the server to shut down and wait for the background thread.
    pub fn shutdown(mut self) {
        // compare_exchange 确保只执行一次
        if self.shutdown_initiated
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .is_ok()
        {
            let _ = self.shutdown_tx.send(true);
            if let Some(handle) = self.server_thread.take() {
                let _ = handle.join();  // 等待线程结束
            }
            info!("Embedded gateway stopped");
        }
    }
}

impl Drop for EmbeddedGatewayHandle {
    fn drop(&mut self) {
        // 仅发送信号，不阻塞等待
        if self.shutdown_initiated
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .is_ok()
        {
            let _ = self.shutdown_tx.send(true);
        }
    }
}
```

**启动函数**：

```rust
/// Start the embedded gateway server on a background thread.
/// Returns a handle that can be used to shutdown the server.
pub fn start_embedded_gateway(
    config: GatewayRuntimeConfig,
) -> Result<EmbeddedGatewayHandle, Box<dyn std::error::Error>> {
    // 1. 端口预绑定（主线程同步）
    let std_listener = TcpListener::bind("127.0.0.1:0")?;
    let port = std_listener.local_addr()?.port();
    let listen_addr: SocketAddr = std_listener.local_addr()?;

    info!("Embedded gateway bound to http://127.0.0.1:{port}");

    // 2. 创建 shutdown channel
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let shutdown_initiated = Arc::new(AtomicBool::new(false));

    // 3. 启动后台线程（独立 tokio runtime）
    let server_thread = std::thread::Builder::new()
        .name("agent-diva-gateway".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async move {
                run_embedded_gateway_task(
                    config, std_listener, listen_addr, shutdown_rx
                ).await;
            });
        })?;

    Ok(EmbeddedGatewayHandle {
        port,
        shutdown_tx,
        server_thread: Some(server_thread),
        shutdown_initiated,
    })
}
```

**后台任务**：

```rust
async fn run_embedded_gateway_task(
    config: GatewayRuntimeConfig,
    std_listener: TcpListener,
    listen_addr: SocketAddr,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    // 1. Bootstrap gateway runtime（复用现有逻辑）
    // 参考 agent-diva-manager/src/runtime.rs 的 run_local_gateway
    
    // 2. 转换 std TcpListener -> tokio TcpListener
    std_listener.set_nonblocking(true).expect("set_nonblocking failed");
    let listener = tokio::net::TcpListener::from_std(std_listener)
        .expect("TcpListener conversion failed");

    // 3. 构建 Router
    let state = AppState { api_tx, bus };  // 从 bootstrap 获取
    let app = build_router(state);

    // 4. 启动 axum server（graceful shutdown）
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.wait_for(|v| *v).await;
            info!("Embedded gateway received shutdown signal");
        })
        .await;
}
```

---

### 2.5 端口文件写入

保持现有 `gateway.port` 文件机制，确保前端兼容：

```rust
// 在 lib.rs setup hook 中调用后
let handle = start_embedded_gateway(config)?;
save_gateway_port_config(handle.port)?;  // 写入端口文件
```

**save_gateway_port_config 实现**（复用现有 commands.rs 中逻辑）：
```rust
fn save_gateway_port_config(port: u16) -> Result<(), String> {
    let config_dir = dirs::data_local_dir()
        .unwrap_or_default()
        .join(".agent-diva");
    std::fs::write(config_dir.join("gateway.port"), port.to_string())
        .map_err(|e| e.to_string())
}
```

---

## 3. 测试验证

### 3.1 编译验证

```bash
cargo build -p agent-diva-manager
cargo build -p agent-diva-gui
```

**预期**：无编译错误，`build_router` 可被 GUI crate 调用。

### 3.2 单元测试（可选）

在 `embedded_server.rs` 中添加测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_binding() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(port > 0);
        assert!(port < 65536);
    }

    #[test]
    fn test_shutdown_initiated_flag() {
        let flag = Arc::new(AtomicBool::new(false));
        let result = flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed);
        assert!(result.is_ok());  // 第一次成功
        let result2 = flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed);
        assert!(result2.is_err());  // 第二次失败（已为 true）
    }
}
```

### 3.3 集成验证

```bash
cargo run -p agent-diva-gui --release
```

**观察点**：
- 日志输出：`Embedded gateway bound to http://127.0.0.1:{port}`
- 端口文件：`~/.agent-diva/gateway.port` 内容为随机端口
- Health check：`curl http://127.0.0.1:{port}/api/health` 返回 200

---

## 4. 关键依赖关系

```
agent-diva-gui/src-tauri/src/embedded_server.rs
  └── agent_diva_manager::build_router(state: AppState)
  └── agent_diva_manager::GatewayRuntimeConfig
  └── agent_diva_manager::AppState

agent-diva-manager/src/server.rs
  └── build_router(state) -> Router
  └── runtime_routes(), provider_routes(), misc_routes()

agent-diva-manager/src/lib.rs
  └── pub use server::build_router;
```

---

## 5. 潜在问题与解决方案

| 问题 | 解决方案 |
|------|----------|
| `run_embedded_gateway_task` 需要完整 bootstrap | 复用 `runtime.rs` 中的 bootstrap 逻辑，或提取为公开函数 |
| `AppState` 需要 `api_tx` 和 `bus` | 从 bootstrap 返回的 GatewayBootstrap 中获取 |
| 独立 runtime 可能与 Tauri 冲突 | 后台线程使用 `std::thread::spawn` + 独立 tokio runtime |

---

## 6. 下一步

Phase 1 完成后，进入 [Phase 2：生命周期整合](./phase2.md)，将内嵌服务器集成到 GUI 启动/退出流程中。