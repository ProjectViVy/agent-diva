# Phase 2：生命周期整合

> 目标：将内嵌 Gateway 集成到 GUI 启动/退出流程，替换子进程管理模式

---

## 1. 步骤概览

| 步骤 | 文件 | 操作 |
|------|------|------|
| 2.1 | `agent-diva-gui/src-tauri/src/lib.rs` | setup hook 重构 |
| 2.2 | `agent-diva-gui/src-tauri/src/lib.rs` | on_window_event 重构 |
| 2.3 | `agent-diva-gui/src-tauri/src/commands.rs` | 移除 GATEWAY_PROCESS，重定向函数 |
| 2.4 | `agent-diva-gui/src-tauri/src/gateway_status.rs` | 新建状态管理结构 |
| 2.5 | 测试验证 | 启动/退出流程验证 |

---

## 2. 详细步骤

### 2.1 Setup Hook 重构

**文件**：`agent-diva-gui/src-tauri/src/lib.rs`

#### 2.1.1 新增模块声明

```rust
// 在文件顶部新增
mod embedded_server;
mod gateway_status;
// process_utils 模块保留但标记 deprecated（后续清理）
```

#### 2.1.2 新增 Managed State 类型

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

// 类型别名，便于使用
type EmbeddedGatewayState = Arc<Mutex<Option<EmbeddedGatewayHandle>>>;
```

#### 2.1.3 替换 Setup Hook 中的子进程启动

**当前代码（lib.rs setup hook）**：
```rust
if should_manage_gateway_lifecycle() {
    // 孤儿进程清理
    let cleanup_result = process_utils::cleanup_orphan_gateway_processes();
    
    // 异步启动子进程（500ms 延迟）
    spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        match commands::start_gateway(app_handle.clone(), None).await {
            Ok(port) => { info!("Gateway auto-started on port {}", port); }
            Err(e) => { error!("Failed to auto-start gateway: {}", e); }
        }
    });
}
```

**改为**：
```rust
if should_manage_gateway_lifecycle() {
    // 1. 构建配置
    let config = build_gateway_runtime_config(&app);
    
    // 2. 启动内嵌服务器（端口预绑定，无延迟）
    let handle = match embedded_server::start_embedded_gateway(config) {
        Ok(h) => h,
        Err(e) => {
            error!("Failed to start embedded gateway: {}", e);
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };
    
    let port = handle.port;
    info!("Embedded gateway started on port {}", port);
    
    // 3. 存储 handle 到 managed state
    let gateway_state: EmbeddedGatewayState = Arc::new(Mutex::new(Some(handle)));
    app.manage(gateway_state.clone());
    
    // 4. 管理 GatewayStatus
    app.manage(GatewayStatus::new(port));
    
    // 5. 端口写入文件（前端兼容）
    save_gateway_port_config(port)?;
}
```

#### 2.1.4 构建配置函数

```rust
fn build_gateway_runtime_config(app: &AppHandle) -> GatewayRuntimeConfig {
    let loader = ConfigLoader::from_default_config_dir();
    let config = loader.load().unwrap_or_default();
    
    GatewayRuntimeConfig {
        config,
        loader,
        workspace: loader.config_dir().parent().unwrap().to_path_buf(),
        cron_store: loader.config_dir().join("cron.json"),
        port: 0,  // 内嵌模式使用预绑定端口，此值被忽略
    }
}
```

#### 2.1.5 移除 500ms 启动延迟

内嵌模式下端口在 setup 阶段已确定，无需等待外部进程就绪，直接移除延迟。

---

### 2.2 On Window Event 重构

**文件**：`agent-diva-gui/src-tauri/src/lib.rs`

**当前代码（on_window_event）**：
```rust
.on_window_event(|window, event| {
    if let tauri::WindowEvent::CloseRequested { api, .. } = &event {
        let app_handle = window.app_handle();
        
        if should_manage_gateway_lifecycle() {
            // 阻止默认关闭
            api.prevent_close();
            
            // 异步停止 gateway
            spawn(async move {
                commands::stop_gateway().await;
                app_handle.exit(0);
            });
        } else {
            // debug 模式直接退出
            app_handle.exit(0);
        }
    }
})
```

**改为**：
```rust
.on_window_event(|window, event| {
    if let tauri::WindowEvent::CloseRequested { api, .. } = &event {
        let app_handle = window.app_handle();
        
        // 检查托盘设置
        let close_to_tray = read_close_to_tray_setting(&app_handle);
        
        if close_to_tray {
            // 隐藏到托盘（服务器继续运行）
            api.prevent_close();
            window.hide().ok();
        } else if should_manage_gateway_lifecycle() {
            // 退出时停止内嵌服务器
            api.prevent_close();
            
            spawn(async move {
                // 获取 handle 并 shutdown
                let gateway_state = app_handle.state::<EmbeddedGatewayState>();
                let mut guard = gateway_state.lock().await;
                if let Some(handle) = guard.take() {
                    handle.shutdown();  // RAII 优雅关闭
                }
                app_handle.exit(0);
            });
        } else {
            // debug 模式直接退出（外部 gateway 不受影响）
            app_handle.exit(0);
        }
    }
})
```

**关键改动**：
- 使用 `handle.shutdown()` 替代 `commands::stop_gateway()`
- 托盘隐藏模式下服务器继续运行
- `guard.take()` 确保 handle 只被消费一次

---

### 2.3 Commands.rs 重构

**文件**：`agent-diva-gui/src-tauri/src/commands.rs`

#### 2.3.1 移除全局静态变量

**当前代码**：
```rust
struct GatewayProcess {
    child: tokio::process::Child,
    executable_path: String,
}

static GATEWAY_PROCESS: Lazy<AsyncMutex<Option<GatewayProcess>>> =
    Lazy::new(|| AsyncMutex::new(None));
```

**改为**：完全移除这些定义（内嵌模式下不再需要）。

#### 2.3.2 重定向 start_gateway

```rust
#[tauri::command]
pub async fn start_gateway(
    _app: AppHandle,
    _bin_path: Option<String>,
) -> Result<u16, String> {
    // 内嵌模式下 gateway 随应用启动，不支持手动启动
    Err("embedded mode: gateway starts automatically with app".to_string())
}
```

#### 2.3.3 重定向 stop_gateway

```rust
#[tauri::command]
pub async fn stop_gateway() -> Result<(), String> {
    // 内嵌模式下通过托盘退出或窗口关闭触发 shutdown
    // 此命令仅用于外部模式（debug）
    Ok(())
}
```

#### 2.3.4 重定向 get_gateway_process_status

```rust
#[tauri::command]
pub async fn get_gateway_process_status(
    state: State<'_, GatewayStatus>,
) -> GatewayStatus {
    state.inner().clone()
}
```

#### 2.3.5 新增 get_gateway_status 命令

```rust
#[tauri::command]
pub fn get_gateway_status(
    state: State<'_, GatewayStatus>,
) -> GatewayStatus {
    state.inner().clone()
}
```

---

### 2.4 新建 gateway_status.rs

**文件**：`agent-diva-gui/src-tauri/src/gateway_status.rs`

```rust
use std::time::Instant;

/// Gateway running status for tray display.
#[derive(Clone, serde::Serialize)]
pub struct GatewayStatus {
    pub port: u16,
    pub running: bool,
    pub started_at: Instant,
}

impl GatewayStatus {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            running: true,
            started_at: Instant::now(),
        }
    }
    
    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }
    
    pub fn format_uptime(&self) -> String {
        let secs = self.uptime_secs();
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m", secs / 60)
        } else {
            format!("{}h", secs / 3600)
        }
    }
    
    pub fn format_status(&self) -> String {
        if self.running {
            format!("Gateway: Running (port: {})", self.port)
        } else {
            "Gateway: Stopped".to_string()
        }
    }
    
    pub fn stop(&mut self) {
        self.running = false;
    }
}
```

---

### 2.5 调整 invoke_handler

**文件**：`agent-diva-gui/src-tauri/src/lib.rs`

```rust
.invoke_handler(tauri::generate_handler![
    // ... 其他命令保持不变
    commands::get_gateway_status,
    commands::get_gateway_process_status,  // 重定向版本
    // 移除或保留 start_gateway/stop_gateway（改为返回错误/noop）
])
```

---

## 3. Debug 模式策略

### 3.1 行为说明

| 模式 | Gateway 行为 | 说明 |
|------|--------------|------|
| **Release** | 内嵌启动 | `should_manage_gateway_lifecycle()` 返回 true |
| **Debug** | 外部依赖 | 不启动内嵌，开发者手动运行 `agent-diva gateway run` |

### 3.2 配置判断函数

保持现有 `should_manage_gateway_lifecycle()` 逻辑：

```rust
fn should_manage_gateway_lifecycle() -> bool {
    // release 模式自动管理
    // debug 模式依赖外部
    !cfg!(debug_assertions)
}
```

### 3.3 开发者指南

Debug 模式下开发流程：

```bash
# 1. 启动外部 gateway（终端 1）
cargo run -p agent-diva-cli -- gateway run

# 2. 启动 GUI（终端 2）
cargo run -p agent-diva-gui

# 3. GUI 会连接到外部 gateway（端口 3000）
```

---

## 4. 测试验证

### 4.1 Release 模式验证

```bash
cargo run -p agent-diva-gui --release
```

**观察点**：
- 日志：`Embedded gateway started on port {random}`
- 端口文件：`~/.agent-diva/gateway.port` 存在
- API：`curl http://127.0.0.1:{port}/api/health` 返回 200

### 4.2 Debug 模式验证

```bash
# 先启动外部 gateway
cargo run -p agent-diva-cli -- gateway run

# 再启动 GUI
cargo run -p agent-diva-gui
```

**观察点**：
- GUI 日志：无 "Embedded gateway" 相关输出
- GUI 连接到外部 gateway 端口 3000

### 4.3 窗口关闭验证

**托盘隐藏模式（close_to_tray = true）**：
- 关闭窗口 → 窗口隐藏，服务器继续运行
- 托盘菜单 → Show Window 恢复窗口

**直接退出模式（close_to_tray = false）**：
- 关闭窗口 → 触发 shutdown → 进程退出

### 4.4 托盘退出验证

- 托盘右键 Quit → 触发 shutdown → 进程退出
- 确认无残留进程（`tasklist` 或 `ps aux`）

---

## 5. 关键改动对比表

| 改动点 | 当前（子进程模式） | 改后（内嵌模式） |
|--------|-------------------|------------------|
| 启动位置 | setup hook async spawn | setup hook 同步调用 |
| 启动延迟 | 500ms | 无（端口预绑定） |
| 进程状态 | GATEWAY_PROCESS 全局静态 | EmbeddedGatewayState managed |
| 停止方式 | commands::stop_gateway() | handle.shutdown() |
| 端口策略 | 固定 3000 + fallback | 随机端口 127.0.0.1:0 |
| Debug 模式 | 不自动启动 | 外部依赖，手动运行 |

---

## 6. 潜在问题与解决方案

| 问题 | 解决方案 |
|------|----------|
| `shutdown()` 在 async spawn 中调用 | 使用 `spawn(async move { ... })` 包装 |
| Drop 可能被调用多次 | AtomicBool compare_exchange 防重入 |
| 前端端口获取时机 | 端口文件在 setup 阶段写入，WebView 加载前已就绪 |

---

## 7. 下一步

Phase 2 完成后，进入 [Phase 3：托盘增强](./phase3.md)，扩展托盘菜单功能。