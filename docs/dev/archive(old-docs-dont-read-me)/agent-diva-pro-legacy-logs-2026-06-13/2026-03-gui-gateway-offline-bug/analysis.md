# agent-diva-gui Mac 打包后“后续打开始终离线”问题分析

## 现象描述

- **首次运行**：正常，后端在线
- **后续打开**：始终显示后端“离线”
- **怀疑点**：启动时和结束时是否缺少“杀历史进程”的能力

## 架构要点

1. **GUI 与 Gateway 关系**：GUI 通过 `start_gateway` 命令 spawn 子进程运行 `agent-diva gateway run`，子进程在 3000 端口启动 API Server
2. **状态存储**：`GATEWAY_PROCESS` 是进程内静态变量，仅保存“本次 GUI 进程”启动的 gateway 子进程句柄
3. **健康检查**：`check_health` 请求 `http://localhost:3000/api/health`，成功则显示“在线”

## 根因分析

### 1. 退出时未清理 Gateway 进程（已确认）

**代码证据**：`agent-diva-gui/src-tauri/src/lib.rs`

```rust
.run(tauri::generate_context!())
.expect("error while running tauri application");
```

- 使用 `.run()` 直接启动，**没有** `RunEvent` 或 `on_exit` 钩子
- 应用退出时**不会**调用 `stop_gateway()`
- Gateway 子进程在 GUI 退出后成为**孤儿进程**，由 launchd 接管

**结论**：退出时确实没有“杀历史进程”的能力。

### 2. 启动时未检测/清理孤儿 Gateway（已确认）

**代码证据**：`refresh_gateway_process_status()` 仅检查 `GATEWAY_PROCESS`

```rust
// commands.rs:2035-2069
async fn refresh_gateway_process_status() -> GatewayProcessStatus {
    let mut guard = GATEWAY_PROCESS.lock().await;
    if let Some(process) = guard.as_mut() {
        // 只检查 GATEWAY_PROCESS 中的子进程
        match process.child.try_wait() { ... }
    } else {
        // GATEWAY_PROCESS 为空时直接返回 "not managed by GUI"
        GatewayProcessStatus {
            running: false,
            details: Some("gateway process is not managed by the GUI".to_string()),
        }
    }
}
```

- 不扫描系统是否存在其他 `agent-diva gateway` 进程
- 不检测 3000 端口是否被占用
- 新 GUI 进程的 `GATEWAY_PROCESS` 必然为空

**结论**：启动时没有检测或清理孤儿进程的能力。

### 3. 端口冲突导致“始终离线”的推演

| 步骤 | 第一次运行 | 第二次运行 |
|-----|-----------|-----------|
| 1 | 用户点击“启动网关” | 用户打开 GUI |
| 2 | `start_gateway` spawn 子进程 | `GATEWAY_PROCESS` 为空 |
| 3 | Gateway 绑定 3000，API 正常 | `check_health` 失败 → 显示离线 |
| 4 | 用户关闭 GUI（未点“停止”） | 用户点击“启动网关” |
| 5 | GUI 退出，**未**调用 `stop_gateway` | `start_gateway` 检查 `running=false`，尝试 spawn |
| 6 | 子进程成为孤儿，继续占用 3000 | 新子进程 bind(3000) **失败**（端口已被占用） |
| 7 | - | Gateway 进程立即退出，`try_wait` 返回已退出 |

**关键点**：若孤儿进程仍在运行并占用 3000，则 `check_health` 理论上应成功。出现“始终离线”可能原因：

- **A**：macOS 在 .app 退出时杀死了子进程（进程组/沙盒行为），导致第二次打开时 3000 上无服务
- **B**：孤儿进程因某种原因崩溃（如 SIGHUP、资源释放），3000 端口空闲，但用户未再次点击“启动网关”
- **C**：用户期望“上次启动过，这次应自动在线”，但 GUI 不会自动启动 gateway，需手动点击

### 4. `start_gateway` 的“已运行”检查缺陷

```rust
// commands.rs:2170-2174
pub async fn start_gateway(...) -> Result<(), String> {
    let current_status = refresh_gateway_process_status().await;
    if current_status.running {
        return Err("gateway process is already running".to_string());
    }
    // ...
}
```

- `running` 仅表示 `GATEWAY_PROCESS` 中有存活的子进程
- 若存在孤儿 gateway 占用 3000，`running` 为 false，会再次 spawn
- 新进程 bind 失败后立即退出，GUI 可能显示“gateway process exited with status X”

### 5. 无自动启动 Gateway

- 代码中**没有**在 `onMounted` 或 `setup` 时自动调用 `start_gateway`
- 用户每次打开 GUI 后需手动在设置页点击“启动网关”
- 若用户未点击，`check_health` 必然失败，显示离线

## 假设汇总

| 假设 | 内容 | 证据强度 |
|-----|------|----------|
| H1 | 退出时未调用 `stop_gateway`，子进程成为孤儿 | 代码明确 |
| H2 | 启动时未检测/清理孤儿进程或占用 3000 的进程 | 代码明确 |
| H3 | 孤儿进程占用 3000 时，新 spawn 会 bind 失败并立即退出 | 逻辑推演 |
| H4 | macOS .app 退出时可能杀死子进程，导致第二次无服务 | 需运行时验证 |
| H5 | 用户未手动点击“启动网关”，导致始终离线 | 行为推演 |

## 建议修复方向

1. **退出时清理**：在 Tauri 中监听 `RunEvent::ExitRequested`，调用 `stop_gateway()` 或等价逻辑（注意 [macOS 上 ExitRequested 可能不触发](https://github.com/tauri-apps/tauri/issues/9198) 的已知问题）
2. **启动时检测**：在 `start_gateway` 前检测 3000 端口或已有 gateway 进程，若存在则先终止再启动，或复用已有进程
3. **可选自动启动**：增加“上次由本 GUI 启动过则本次自动启动”的配置，减少用户操作

## 相关代码位置

- Gateway 启动/停止：`agent-diva-gui/src-tauri/src/commands.rs` 第 2169–2228 行
- 状态刷新：`refresh_gateway_process_status` 第 2035–2070 行
- 应用入口：`agent-diva-gui/src-tauri/src/lib.rs` 第 43–128 行
- 健康检查：`check_health` 请求 `http://localhost:3000/api/health`
