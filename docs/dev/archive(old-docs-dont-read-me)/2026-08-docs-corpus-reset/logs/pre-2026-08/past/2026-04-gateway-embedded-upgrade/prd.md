# 技术简报：GUI 内嵌 Gateway 改造（方案 A）

## 1. 项目背景

### 当前状态
- **agent-diva-gui** 启动方式：GUI 进程 spawn 外部 gateway 子进程
- **子进程管理**：通过 `process_utils.rs` 实现 PID 检测、端口扫描、进程清理
- **生命周期**：release 模式自动启动 gateway，debug 模式手动运行
- **系统托盘**：已实现基础功能（窗口隐藏、退出）

### 问题痛点
- 子进程管理复杂：跨平台进程检测、清理、端口冲突处理
- 启动时序问题：需要等待外部进程就绪，延迟不可控
- 资源清理风险：异常退出时可能遗留孤儿进程
- 调试体验差：debug/release 行为不一致

### 目标方向
借鉴 **openfang-desktop** 架构，改为 GUI 内嵌 Gateway，消除子进程依赖。

---

## 1.1 现状分析（详细）

### 1.1.1 当前 GUI 启动架构

**核心文件**：`agent-diva-gui/src-tauri/src/lib.rs`

```rust
// lib.rs 当前启动逻辑
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // 生命周期管理判断
            if should_manage_gateway_lifecycle() {  // release 模式为 true
                // 1. 清理孤儿进程
                let cleanup_result = process_utils::cleanup_orphan_gateway_processes();
                
                // 2. 异步启动 gateway 子进程
                spawn(async move {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    match commands::start_gateway(app_handle.clone(), None).await {
                        Ok(port) => { info!("Gateway auto-started on port {}", port); }
                        Err(e) => { error!("Failed to auto-start gateway: {}", e); }
                    }
                });
            }
            // 3. 初始化系统托盘
            tray::init_tray(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            // 窗口关闭事件
            if let tauri::WindowEvent::CloseRequested { api, .. } = &event {
                if should_manage_gateway_lifecycle() {
                    spawn(async move {
                        commands::stop_gateway().await;  // 终止子进程
                        app_handle.exit(0);
                    });
                }
            }
        })
}
```

**关键特点**：
- `should_manage_gateway_lifecycle()`：release 模式自动管理，debug 模式手动
- 子进程启动延迟 500ms（等待 GUI 初始化）
- 退出时同步终止 gateway 子进程

### 1.1.2 当前 Gateway 子进程管理

**核心文件**：`agent-diva-gui/src-tauri/src/commands.rs:2281-2378`

```rust
// start_gateway 实现
#[tauri::command]
pub async fn start_gateway(app: AppHandle, bin_path: Option<String>) -> Result<u16, String> {
    // 1. 检查进程是否已运行
    let current_status = refresh_gateway_process_status().await;
    if current_status.running {
        return Err("gateway process is already running".to_string());
    }

    // 2. 端口策略（三重策略）
    let port = if process_utils::is_port_3000_occupied() {
        // Strategy 1: 清理冲突进程
        process_utils::force_cleanup_all_gateway_processes().await;
        // Strategy 2: 等待端口释放
        match process_utils::wait_for_port_available(5, 3000).await {
            Ok(true) => 3000,
            Ok(false) => {
                // Strategy 3: 回退到动态端口
                process_utils::find_first_available_port(3001, 3010)
                    .unwrap_or_else(|| return Err("All ports unavailable"))
            }
        }
    } else {
        3000
    };

    // 3. 启动 CLI gateway run 子进程
    let executable = resolved_cli_binary_for_launch(&app, bin_path)?;
    let mut command = TokioCommand::new(&executable);
    configure_background_command(&mut command);  // Windows: CREATE_NO_WINDOW
    command
        .arg("--config-dir")
        .arg(loader.config_dir())
        .arg("gateway")
        .arg("run")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = command.spawn()?;
    
    // 4. 保存进程句柄
    *GATEWAY_PROCESS.lock().await = Some(GatewayProcess {
        child,
        executable_path: executable.display().to_string(),
    });

    // 5. 持久化端口配置
    save_gateway_port_config(port)?;
    Ok(port)
}
```

**进程状态存储**：

```rust
// commands.rs:26-32
struct GatewayProcess {
    child: tokio::process::Child,
    executable_path: String,
}

static GATEWAY_PROCESS: Lazy<AsyncMutex<Option<GatewayProcess>>> =
    Lazy::new(|| AsyncMutex::new(None));
```

### 1.1.3 当前端口/进程检测工具

**核心文件**：`agent-diva-gui/src-tauri/src/process_utils.rs`

| 函数 | 实现方式 | 用途 |
|------|----------|------|
| `is_port_3000_occupied()` | Windows: `netstat -ano`，Unix: `lsof/netstat` | 端口占用检测 |
| `find_gateway_processes()` | Windows: `tasklist /FI`，Unix: `pgrep -f` | 进程查找 |
| `terminate_process(pid)` | Windows: `taskkill /F /PID`，Unix: `kill -9` | 进程终止 |
| `wait_for_port_available()` | 指数退避轮询（100ms base） | 等待端口释放 |
| `find_first_available_port()` | 顺序扫描 3001-3010 | 动态端口回退 |

**关键问题**：
- 无 PID 健康检查（进程退出后 PID 可能被 reuse）
- 无 TCP 响应检查（端口可能被其他进程占用）
- 孤儿进程清理依赖 `tasklist/pgrep`，跨平台差异大

### 1.1.4 当前 Gateway 运行时架构

**核心文件**：`agent-diva-manager/src/runtime.rs`

```rust
// Gateway 运行时入口
pub async fn run_local_gateway(runtime: GatewayRuntimeConfig) -> Result<()> {
    let port = runtime.port;
    
    // 1. Bootstrap 阶段
    let bootstrap = bootstrap::bootstrap_runtime(runtime).await?;
    
    // 2. Channel Bootstrap
    let channel_bootstrap = bootstrap::bootstrap_channel_runtime(
        &bootstrap.config, 
        bootstrap.bus.clone()
    ).await;
    
    // 3. 启动运行时任务
    let mut tasks = task_runtime::start_runtime_tasks(bootstrap, channel_bootstrap).await;
    
    // 4. 等待 shutdown 信号
    tracing::info!("Gateway ready; HTTP API at http://127.0.0.1:{}", port);
    let manager_handle_completed = shutdown::wait_for_shutdown(&mut tasks).await;
    
    // 5. 执行 shutdown 清理
    shutdown::shutdown_runtime(tasks, manager_handle_completed).await;
    Ok(())
}
```

**GatewayTasks 结构**：

```rust
// runtime.rs:59-72
struct GatewayTasks {
    bus: MessageBus,
    cron_service: Arc<CronService>,
    channel_manager: Arc<ChannelManager>,
    server_shutdown_tx: broadcast::Sender<()>,  // HTTP server shutdown
    inbound_bridge_handle: JoinHandle<()>,
    outbound_dispatch_handle: JoinHandle<()>,
    channel_handle: JoinHandle<()>,
    agent_handle: JoinHandle<()>,
    manager_handle: JoinHandle<Result<()>>,
    server_handle: JoinHandle<()>,
}
```

**Shutdown 信号源**（`shutdown.rs`）：

```rust
pub(super) async fn wait_for_shutdown(tasks: &mut GatewayTasks) -> bool {
    tokio::select! {
        // 1. Ctrl+C 信号
        _ = tokio::signal::ctrl_c() => {}
        
        // 2. Manager 任务结束
        res = &mut tasks.manager_handle => { ... }
    }
}
```

**关键问题**：
- `broadcast::channel` 仅用于 HTTP server，无统一 Supervisor
- Shutdown 通过 `abort()` 强制终止各任务，非优雅关闭
- 无状态持久化（Agent 状态不会保存为 Suspended）

### 1.1.5 当前 HTTP Server 实现

**核心文件**：`agent-diva-manager/src/server.rs`

```rust
pub async fn run_server(
    state: AppState,
    port: u16,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    let app = build_app(state);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    // Graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.recv().await;
            tracing::info!("Server shutting down signal received");
        })
        .await?;
    Ok(())
}
```

**关键问题**：
- 无 `SO_REUSEADDR` 配置（重启时可能 TIME_WAIT）
- 无端口预绑定机制（依赖外部传入 port）
- `build_app()` 未暴露为公开 API（需改造）

### 1.1.6 当前 AgentState 与 API 连接

**核心文件**：`agent-diva-gui/src-tauri/src/app_state.rs`

```rust
#[derive(Clone)]
pub struct AgentState {
    pub client: reqwest::Client,
    pub api_base_url: String,
    pub gateway_port: u16,
}

impl AgentState {
    pub fn new() -> Self {
        let gateway_port = Self::load_gateway_port();  // 从 gateway.port 文件读取
        Self {
            // 关键：使用 .no_proxy() 防止系统代理干扰
            client: reqwest::Client::builder()
                .no_proxy()
                .build()
                .expect("reqwest client"),
            api_base_url: format!("http://127.0.0.1:{}/api", gateway_port),
            gateway_port,
        }
    }
}
```

### 1.1.7 现状问题总结

| 问题分类 | 具体问题 | 影响 |
|----------|----------|------|
| **架构层面** | GUI spawn 子进程模式 | 进程管理复杂、时序不可控 |
| **端口管理** | 固定端口 3000 + 回退 | 端口冲突处理繁琐 |
| **进程检测** | 无 PID alive + TCP 响应检查 | Stale PID 文件风险 |
| **生命周期** | broadcast 仅用于 HTTP server | 无统一优雅关闭机制 |
| **状态持久化** | Shutdown 时无状态保存 | Agent 状态丢失 |
| **Server API** | `build_app()` 未公开 | 无法直接内嵌调用 |
| **跨平台** | 多种 Win32/Unix 命令 | 维护成本高 |

---

## 2. 具体需求

### 2.1 GUI 启动时内嵌 Gateway（端口预绑定）
- Gateway 作为 GUI 进程内部组件运行，不 spawn 外部子进程
- 使用 `127.0.0.1:0` 预绑定随机端口，启动前确定端口
- 端口信息写入 `gateway.port` 文件，供前端/API 使用

### 2.2 启动动画
- 保留当前 splash screen 机制（已实现）
- 简化方案 A：无需 Q/L 交互键，自动等待服务器就绪后关闭 splash
- 就绪检测：内部状态标志，而非外部进程健康检查

### 2.3 Gateway 后台运行
- GUI 窗口关闭时隐藏到系统托盘（已实现）
- 托盘菜单：显示窗口、退出应用（可扩展：打开日志目录等）
- 退出时优雅关闭内嵌服务器（RAII Drop）

### 2.4 TUI 日志模式（可选）
- 统一日志输出可集成 TUI 终端界面
- 作为后续增强功能，本次不作为核心交付

### 2.5 CLI 命令统一
- 当前已满足：`agent-diva gateway` 等命令
- 无需改动

---

## 3. 实现方案

### 3.1 参考架构：openfang-desktop

核心设计模式（参见 `.workspace/openfang/crates/openfang-desktop/src/server.rs`）：

```rust
pub struct ServerHandle {
    pub port: u16,
    pub kernel: Arc<OpenFangKernel>,
    shutdown_tx: watch::Sender<bool>,
    server_thread: Option<std::thread::JoinHandle<()>>,
    shutdown_initiated: Arc<AtomicBool>,
}
```

关键流程：
1. **主线程端口预绑定**：`TcpListener::bind("127.0.0.1:0")` → 端口确定
2. **后台线程启动服务器**：独立 tokio runtime，运行 axum server
3. **RAII 生命周期**：Drop 时发送 shutdown 信号，优雅退出
4. **watch channel**：shutdown 信号通过 `tokio::sync::watch` 传递

### 3.2 关键改造点

#### 3.2.1 ServerHandle RAII 模式
- 新建 `agent-diva-gui/src-tauri/src/embedded_server.rs`
- 实现 `ServerHandle` 结构体，持有：
  - `port: u16`
  - `shutdown_tx: watch::Sender<bool>`
  - `server_thread: JoinHandle<()>`
- Drop trait 触发优雅关闭

#### 3.2.2 端口预绑定
- `lib.rs` setup 阶段调用 `start_embedded_server()`
- 返回 `ServerHandle`，存储于 `app.manage()`
- 端口写入 `gateway.port`（与当前机制兼容）

#### 3.2.3 内嵌启动
- 移除 `process_utils.rs` 中的子进程 spawn 逻辑
- `commands.rs` 中的 `start_gateway/stop_gateway` 重定向到内嵌服务器控制
- debug 模式行为：可选择内嵌或外部（保留灵活性）

#### 3.2.4 系统托盘增强
- 当前 `tray.rs` 已有基础实现
- 增加：服务器状态显示（运行/停止）
- 增加：打开日志目录菜单项

### 3.3 依赖关系调整

```
agent-diva-gui (Tauri)
├── agent-diva-manager (HTTP API router)
│   └── agent-diva-core
│   └── agent-diva-agent
│   └── agent-diva-providers
└── embedded_server.rs (新增)
    └── ServerHandle (RAII)
```

关键：`agent-diva-manager` 的 `build_router()` 需暴露为库函数供 GUI 调用。

---

## 4. 分阶段实施建议

### Phase 1：基础设施（MVP）
1. 新建 `embedded_server.rs`，实现 `ServerHandle` 和 `start_embedded_server()`
2. 修改 `agent-diva-manager` 暴露 `build_router()` 公开接口
3. `lib.rs` 替换子进程启动为内嵌服务器启动
4. 验证端口预绑定和基本功能

### Phase 2：生命周期管理
1. Drop trait 优雅关闭
2. 窗口关闭 → 隐藏到托盘（已有）
3. 托盘退出 → 触发服务器关闭
4. 移除 `process_utils.rs` 中孤儿进程清理逻辑

### Phase 3：体验优化
1. Splash screen 就绪检测简化
2. 托盘菜单增强（状态显示、日志目录）
3. debug 模式策略明确（内嵌 vs 外部）

### Phase 4：可选功能
1. TUI 日志模式集成（独立特性）
2. 自动更新机制（参考 openfang updater.rs）

---

## 5. 注意事项

### 5.1 技术风险

| 风险项 | 影响 | 缓解措施 |
|--------|------|----------|
| `agent-diva-manager` 当前为 CLI 设计 | 需重构为库模式 | 分离 main.rs 与 lib.rs，暴露 API |
| tokio runtime 嵌套 | 可能阻塞或冲突 | 使用独立 runtime（参考 openfang） |
| 窗口事件处理时机 | Drop 时序不确定 | 使用 `shutdown_initiated` AtomicBool 防重入 |
| 跨平台端口绑定 | Windows/Linux 行为差异 | 使用标准 `TcpListener::bind` |

### 5.2 兼容性考虑

- **CLI 用户**：`agent-diva gateway` 命令继续独立运行，不受影响
- **配置文件**：`gateway.port` 机制不变，前端读取方式兼容
- **API 路径**：`/api/*` 路由不变
- **debug 模式**：保留开发灵活性，可配置内嵌或外部

### 5.3 参考文件路径

- openfang 桌面架构文档：`.workspace/openfang/repowiki/zh/content/桌面应用.md`
- openfang server.rs：`.workspace/openfang/crates/openfang-desktop/src/server.rs`
- 当前 GUI lib.rs：`agent-diva-gui/src-tauri/src/lib.rs`
- 当前 GUI process_utils.rs：`agent-diva-gui/src-tauri/src/process_utils.rs`
- 当前 GUI tray.rs：`agent-diva-gui/src-tauri/src/tray.rs`
- agent-diva-manager：`agent-diva-manager/src/main.rs`（需拆分 lib.rs）

---

## 6. 验收标准

- [ ] GUI 启动后内嵌服务器运行在随机端口
- [ ] 端口写入 `gateway.port`，前端正常连接
- [ ] 窗口关闭隐藏到托盘，服务器继续运行
- [ ] 托盘退出时服务器优雅关闭，无残留进程
- [ ] debug 模式行为可控（文档说明）
- [ ] 跨平台测试：Windows + Linux（如条件允许）

---

## 7. 附录：架构对比

### 当前架构（子进程模式）

```
┌─────────────────┐     spawn      ┌─────────────────┐
│   GUI (Tauri)   │ ──────────────>│ Gateway 子进程  │
│                 │                 │ (agent-diva)    │
│  process_utils  │<───────────────│ HTTP API :3000  │
│  PID/端口检测    │    healthck    │                 │
└─────────────────┘                 └─────────────────┘
```

### 目标架构（内嵌模式）

```
┌─────────────────────────────────────┐
│           GUI (Tauri)               │
│  ┌───────────────────────────────┐  │
│  │    ServerHandle (RAII)        │  │
│  │    ├─ port: u16               │  │
│  │    ├─ shutdown_tx             │  │
│  │    └─ server_thread           │  │
│  └───────────────────────────────┘  │
│              │                      │
│              ▼                      │
│  ┌───────────────────────────────┐  │
│  │  后台线程 (tokio runtime)     │  │
│  │  ├─ agent-diva-manager 路由   │  │
│  │  └─ axum server :{port}       │  │
│  └───────────────────────────────┘  │
│                                     │
│  WebView ──> http://127.0.0.1:{port}│
└─────────────────────────────────────┘
```

---

*文档版本：v0.1.0*
*创建日期：2026-04-17*