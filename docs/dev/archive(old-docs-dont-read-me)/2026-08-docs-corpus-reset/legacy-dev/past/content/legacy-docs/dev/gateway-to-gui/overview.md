# GUI 内嵌 Gateway 改造 - 总体架构说明

> 文档版本：v1.0.0  
> 创建日期：2026-04-17  
> 关联 PRD：[docs/logs/2026-04-gateway-embedded-upgrade/prd.md](../../logs/2026-04-gateway-embedded-upgrade/prd.md)

---

## 1. 项目背景

### 1.1 当前架构（子进程模式）

```
┌─────────────────┐     spawn      ┌─────────────────┐
│   GUI (Tauri)   │ ──────────────>│ Gateway 子进程  │
│                 │                 │ (agent-diva)    │
│  process_utils  │<───────────────│ HTTP API :3000  │
│  PID/端口检测    │    healthck    │                 │
└─────────────────┘                 └─────────────────┘
```

**当前问题痛点**：

| 问题 | 影响 | 根因 |
|------|------|------|
| 子进程管理复杂 | 维护成本高 | 跨平台进程检测/清理（netstat、tasklist、pgrep） |
| 启动时序不可控 | 用户体验差 | 依赖 500ms 延迟等待外部进程就绪 |
| 孤儿进程风险 | 资源泄漏 | 异常退出时可能遗留 gateway 进程 |
| Debug/Release 行为不一致 | 调试困难 | release 自动管理，debug 手动控制 |

### 1.2 目标架构（内嵌模式）

```
┌─────────────────────────────────────────────────────────────┐
│                   GUI (Tauri 主进程)                         │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              EmbeddedGatewayHandle (RAII)           │   │
│  │  ├─ port: u16                                       │   │
│  │  ├─ shutdown_tx: watch::Sender<bool>                │   │
│  │  ├─ server_thread: Option<std::thread::JoinHandle> │   │
│  │  ├─ shutdown_initiated: Arc<AtomicBool>            │   │
│  └─────────────────────────────────────────────────────┘   │
│                         │                                   │
│                         ▼                                   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │           后台线程 (独立 tokio runtime)             │   │
│  │  ├─ agent-diva-manager 路由                         │   │
│  │  ├─ axum server :{random_port}                      │   │
│  │  ├─ Agent Loop                                      │   │
│  │  ├─ Channel Manager                                  │   │
│  │  └─ Cron Service                                     │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  WebView ──────> http://127.0.0.1:{port}/api               │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │               System Tray (增强)                     │   │
│  │  ├─ Show Window                                      │   │
│  │  ├─ Gateway Status: Running ({port})               │   │
│  │  ├─ Open Config Directory                            │   │
│  │  ├─ Open Logs Directory                              │   │
│  │  └─ Quit                                             │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. 核心设计决策

| 决策点 | 选择 | 原因 |
|--------|------|------|
| **端口分配** | 主线程同步绑定 `127.0.0.1:0` | 随机端口，无冲突风险，启动前已确定 |
| **Shutdown 机制** | `watch::Sender<bool>` + AtomicBool | 参考 openfang-desktop，防重入关闭 |
| **Runtime 隔离** | 后台线程独立 tokio runtime | 与 Tauri runtime 達隔离，避免调度冲突 |
| **生命周期管理** | RAII Drop + 显式 shutdown() | 双重保障，支持优雅退出 |
| **API 暴露** | manager lib.rs 新增 `build_router()` | 复用现有路由，避免重复实现 |
| **Debug 模式** | 保持外部依赖模式 | 开发灵活性，可独立调试 gateway |

---

## 3. 改造范围

### 3.1 需要修改的文件

| 文件 | 改动类型 | 改动内容 |
|------|----------|----------|
| `agent-diva-manager/src/server.rs` | 修改 | `build_app` → `build_router` (pub) |
| `agent-diva-manager/src/lib.rs` | 修改 | 新增导出 `build_router` |
| `agent-diva-gui/src-tauri/src/lib.rs` | 修改 | setup hook 重构，移除子进程启动 |
| `agent-diva-gui/src-tauri/src/commands.rs` | 修改 | 移除 GATEWAY_PROCESS，重定向函数 |
| `agent-diva-gui/src-tauri/src/tray.rs` | 修改 | 扩展托盘菜单（状态显示、目录打开） |
| `agent-diva-gui/src-tauri/Cargo.toml` | 修改 | 新增依赖 agent-diva-manager |

### 3.2 需要新建的文件

| 文件 | 内容 |
|------|------|
| `agent-diva-gui/src-tauri/src/embedded_server.rs` | RAII ServerHandle 实现 |
| `agent-diva-gui/src-tauri/src/gateway_status.rs` | Gateway 状态管理结构 |

---

## 4. 分阶段实施计划

### Phase 1：基础设施改造（MVP）
- 暴露 agent-diva-manager 路由构建 API
- 创建 embedded_server.rs 实现 RAII ServerHandle
- 端口预绑定和后台服务器启动
- 验证基本功能

**详细文档**：[phase1.md](./phase1.md)

### Phase 2：生命周期整合
- 替换 setup hook 中的子进程启动
- 替换 on_window_event 中的子进程停止
- 移除全局静态变量 GATEWAY_PROCESS
- 保留 debug 模式外部依赖能力

**详细文档**：[phase2.md](./phase2.md)

### Phase 3：托盘增强与体验优化
- 托盘菜单增加 Gateway 状态显示
- 托盘菜单增加"打开配置目录"、"打开日志目录"
- Splash screen 就绪检测简化

**详细文档**：[phase3.md](./phase3.md)

### Phase 4：清理与最终验证
- 清理遗留 process_utils.rs 中不再需要的函数
- 移除或标记 deprecated 相关命令
- 完整集成测试和跨平台验证

**详细文档**：[phase4.md](./phase4.md)

---

## 5. 参考架构

本次改造参考 `.workspace/openfang/crates/openfang-desktop/src/server.rs` 的设计模式：

- **ServerHandle RAII**：持有 port、shutdown_tx、server_thread、shutdown_initiated AtomicBool
- **端口预绑定**：TcpListener::bind("127.0.0.1:0") 主线程同步绑定
- **独立 Tokio runtime**：后台线程创建专属 runtime，与 Tauri 達隔离
- **watch channel shutdown**：axum with_graceful_shutdown 集成
- **AtomicBool compare_exchange**：防重入关闭保护

**详细对比分析**：[reference.md](./reference.md)

---

## 6. 验收标准

| 验收项 | 验证方法 | 预期结果 |
|--------|----------|----------|
| Release 启动 | `cargo run --release` | 内嵌服务器启动，随机端口 |
| 端口文件 | 检查 `gateway.port` | 文件存在，内容为端口 |
| Health check | curl `/api/health` | 返回 200 OK |
| 窗口隐藏 | 关闭窗口 | 隐藏到托盘，服务器继续 |
| 托盘退出 | 右键 Quit | 服务器关闭，进程退出 |
| 托盘状态 | 右键菜单 | 显示 Gateway Running (port) |
| 多次启动 | 连续启动/退出 | 无端口冲突，无孤儿进程 |
| CLI 独立 | `agent-diva gateway run` | 继续正常运行，不受影响 |

---

## 7. 技术风险

| 风险项 | 影响 | 缓解措施 |
|--------|------|----------|
| Tokio runtime 嵌套冲突 | 可能 panic 或阻塞 | 后台线程独立 runtime |
| Drop 时序不确定 | shutdown 可能重复调用 | AtomicBool compare_exchange 防重入 |
| 优雅关闭超时 | 某些任务长时间阻塞 | 设置 5s timeout，超时后 abort |
| 跨平台兼容 | Windows/Linux 行为差异 | 使用标准 TcpListener::bind |

---

## 8. 文档索引

| 文档 | 内容 |
|------|------|
| [overview.md](./overview.md) | 总体架构说明（本文档） |
| [phase1.md](./phase1.md) | Phase 1 基础设施改造 |
| [phase2.md](./phase2.md) | Phase 2 生命周期整合 |
| [phase3.md](./phase3.md) | Phase 3 托盘增强 |
| [phase4.md](./phase4.md) | Phase 4 清理与验证 |
| [reference.md](./reference.md) | 参考架构对比分析 |