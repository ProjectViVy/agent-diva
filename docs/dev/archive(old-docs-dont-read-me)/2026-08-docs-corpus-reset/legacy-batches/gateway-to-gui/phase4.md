# Phase 4：清理与最终验证

> 目标：清理遗留代码，完成集成测试和跨平台验证

---

## 1. 步骤概览

| 步骤 | 文件 | 操作 |
|------|------|------|
| 4.1 | `agent-diva-gui/src-tauri/src/process_utils.rs` | 标记 deprecated |
| 4.2 | `agent-diva-gui/src-tauri/src/commands.rs` | 清理废弃函数 |
| 4.3 | `agent-diva-gui/src-tauri/src/lib.rs` | 移除 process_utils 引用 |
| 4.4 | 集成测试 | 完整功能验证 |
| 4.5 | 跨平台验证 | Windows/Linux 测试 |
| 4.6 | 文档更新 | README 和 ARCHITECTURE 更新 |

---

## 2. 详细步骤

### 2.1 清理 process_utils.rs

**文件**：`agent-diva-gui/src-tauri/src/process_utils.rs`

#### 2.1.1 处理方案

保留文件但标记所有函数为 deprecated，函数体改为 noop 或 warning log。

**改动示例**：

```rust
/// Process utilities for external gateway management.
/// DEPRECATED: Use embedded gateway instead.
/// These functions are retained for debug mode compatibility only.

#[deprecated(
    note = "Use embedded gateway instead. Only needed for debug mode external gateway."
)]
pub fn cleanup_orphan_gateway_processes() -> usize {
    tracing::warn!("cleanup_orphan_gateway_processes is deprecated in embedded mode");
    0
}

#[deprecated(
    note = "Use embedded gateway instead. Only needed for debug mode external gateway."
)]
pub fn is_port_3000_occupied() -> bool {
    tracing::warn!("is_port_3000_occupied is deprecated in embedded mode");
    false
}

#[deprecated(
    note = "Use embedded gateway instead. Only needed for debug mode external gateway."
)]
pub fn find_gateway_processes() -> Vec<u32> {
    tracing::warn!("find_gateway_processes is deprecated in embedded mode");
    vec![]
}

#[deprecated(
    note = "Use embedded gateway instead. Only needed for debug mode external gateway."
)]
pub fn terminate_process(pid: u32) -> bool {
    tracing::warn!("terminate_process is deprecated in embedded mode");
    false
}

#[deprecated(
    note = "Use embedded gateway instead. Only needed for debug mode external gateway."
)]
pub async fn force_cleanup_all_gateway_processes() {
    tracing::warn!("force_cleanup_all_gateway_processes is deprecated in embedded mode");
}

#[deprecated(
    note = "Use embedded gateway instead. Only needed for debug mode external gateway."
)]
pub fn find_first_available_port(start: u16, end: u16) -> Option<u16> {
    tracing::warn!("find_first_available_port is deprecated in embedded mode");
    None
}

#[deprecated(
    note = "Use embedded gateway instead. Only needed for debug mode external gateway."
)]
pub async fn wait_for_port_available(max_attempts: u32, port: u16) -> Result<bool, String> {
    tracing::warn!("wait_for_port_available is deprecated in embedded mode");
    Ok(false)
}
```

#### 2.1.2 保留原因

- Debug 模式下开发者可能需要手动检测外部 gateway 进程
- 向后兼容：避免意外 break 现有代码调用

---

### 2.2 清理 commands.rs

**文件**：`agent-diva-gui/src-tauri/src/commands.rs`

#### 2.2.1 移除或标记废弃函数

```rust
// 移除（内嵌模式下无意义）
// - uninstall_gateway 命令
// - GatewayProcess 结构体
// - GATEWAY_PROCESS 全局静态变量

// 标记 deprecated（改为返回错误）
#[deprecated(note = "Embedded mode: gateway starts automatically")]
#[tauri::command]
pub async fn start_gateway(...) -> Result<u16, String> {
    Err("embedded mode: gateway starts automatically with app".to_string())
}

// service 相关命令保留（用于 Windows 服务管理，独立功能）
#[tauri::command]
pub async fn install_service(...) -> Result<(), String> { ... }

#[tauri::command]
pub async fn uninstall_service(...) -> Result<(), String> { ... }
```

#### 2.2.2 invoke_handler 清理

```rust
.invoke_handler(tauri::generate_handler![
    // 保留
    commands::get_gateway_status,
    commands::get_config,
    commands::update_config,
    commands::check_health,
    commands::get_sessions,
    // ... 其他正常命令
    
    // 移除或改为 deprecated wrapper
    #[allow(deprecated)]
    commands::start_gateway,  // 如果需要保留兼容性
    #[allow(deprecated)]
    commands::stop_gateway,
    
    // 移除
    // commands::uninstall_gateway,  // 无意义，移除
])
```

---

### 2.3 lib.rs 引用清理

**文件**：`agent-diva-gui/src-tauri/src/lib.rs`

```rust
// 移除或标记 deprecated 引用
#[allow(deprecated)]
use process_utils::cleanup_orphan_gateway_processes;

// setup hook 中移除调用
// 当前代码
if should_manage_gateway_lifecycle() {
    let _cleanup = cleanup_orphan_gateway_processes();  // 移除
    ...
}

// 改为
if should_manage_gateway_lifecycle() {
    // 内嵌模式无需孤儿进程清理
    tracing::info!("Starting embedded gateway...");
    ...
}
```

---

### 2.4 集成测试清单

### 2.4.1 Release 模式完整验证

| 测试项 | 命令/操作 | 预期结果 |
|--------|----------|----------|
| 编译 | `cargo build --release` | 无错误无警告 |
| 启动 | `cargo run --release` | 内嵌服务器启动 |
| 端口文件 | `cat ~/.agent-diva/gateway.port` | 随机端口数字 |
| Health API | `curl http://127.0.0.1:{port}/api/health` | 返回 200 OK |
| 窗口隐藏 | 关闭窗口（close_to_tray=true） | 窗口隐藏，服务器继续 |
| 窗口恢复 | 托盘 Show Window | 窗口显示 |
| 托盘状态 | 右键菜单查看 | 显示 "Gateway: Running (port: xxx)" |
| 配置目录 | 点击菜单项 | 打开正确目录 |
| 日志目录 | 点击菜单项 | 打开 logs 子目录 |
| 托盘退出 | 点击 Quit | 服务器关闭，进程退出 |
| 无残留进程 | `tasklist` 或 `ps aux` | 无 agent-diva 相关进程 |

### 2.4.2 Debug 模式验证

| 测试项 | 命令/操作 | 预期结果 |
|--------|----------|----------|
| 外部 gateway | `cargo run -p agent-diva-cli -- gateway run` | 端口 3000 |
| GUI 启动 | `cargo run -p agent-diva-gui` | 连接外部 gateway |
| GUI 退出 | 关闭窗口 | GUI 退出，gateway 继续运行 |

### 2.4.3 多次启动/退出验证

```bash
# 循环测试
for i in {1..5}; do
    cargo run --release &
    sleep 5
    curl http://127.0.0.1:{port}/api/health
    # 托盘退出
    sleep 2
done

# 确认每次都使用不同随机端口
# 确认每次退出后无残留进程
```

### 2.4.4 异常退出验证

```bash
# 模拟强制终止
cargo run --release &
PID=$!
sleep 5
kill -9 $PID

# 等待几秒后检查残留进程
sleep 3
tasklist | grep agent-diva  # 应无结果
```

---

### 2.5 跨平台验证

### 2.5.1 Windows 验证

| 验证项 | 注意事项 |
|--------|----------|
| 端口绑定 | Windows Defender 可能拦截首次启动 |
| 托盘菜单 | 系统托盘位置和右键行为 |
| 目录打开 | `explorer.exe` 打开 |
| 进程残留 | `tasklist` 检查 |

**测试命令**：
```powershell
cargo run --release
# 启动后检查
netstat -ano | findstr LISTENING | findstr agent
tasklist | findstr agent-diva
```

### 2.5.2 Linux 验证

| 验证项 | 注意事项 |
|--------|----------|
| 端口绑定 | AppImage 权限问题 |
| 托盘菜单 | GNOME/KDE 托盘支持 |
| 目录打开 | `xdg-open` 依赖 |
| 进程残留 | `ps aux` 检查 |

**测试命令**：
```bash
cargo run --release
# 启动后检查
ps aux | grep agent-diva
lsof -i :{port}
```

### 2.5.3 macOS 验证（如条件允许）

| 验证项 | 注意事项 |
|--------|----------|
| 端口绑定 | 通常无问题 |
| 托盘菜单 | 系统托盘位置 |
| 目录打开 | Finder 打开 |
| 进程残留 | `ps aux` 检查 |

---

### 2.6 文档更新

### 2.6.1 GUI README 更新

**文件**：`agent-diva-gui/src-tauri/README.md`（如存在）

新增内容：
```markdown
## Gateway Architecture

### Embedded Gateway Mode (Release)

In release builds, the gateway HTTP server runs embedded within the GUI process:
- Port: Random (127.0.0.1:0), written to `gateway.port`
- Lifecycle: Managed by GUI (RAII handle)
- Shutdown: Graceful via tray Quit or window close

### External Gateway Mode (Debug)

In debug builds, the GUI expects an external gateway process:
- Run: `cargo run -p agent-diva-cli -- gateway run`
- Port: 3000 (default)
- GUI connects to external gateway

### System Tray Features

- Show Window: Restore hidden window
- Gateway Status: Display running state and port
- Open Config Directory: Open ~/.agent-diva
- Open Logs Directory: Open logs subfolder
- Quit: Graceful shutdown
```

### 2.6.2 项目架构文档更新

**文件**：`docs/dev/architecture.md`（如存在）

新增章节：
```markdown
## GUI Gateway Architecture

### Embedded Mode (v2.0+)

The GUI embeds the gateway server using RAII ServerHandle pattern:
- Port pre-binding (TcpListener::bind("127.0.0.1:0"))
- Independent tokio runtime on background thread
- Graceful shutdown via watch channel

### Key Files

- `embedded_server.rs`: RAII ServerHandle implementation
- `gateway_status.rs`: Running state tracking
- `lib.rs`: Startup/shutdown lifecycle
```

---

## 3. 验收标准汇总

| 验收项 | 状态 |
|--------|------|
| Release 启动内嵌服务器在随机端口 | [ ] |
| 端口写入 `gateway.port`，前端正常连接 | [ ] |
| 窗口关闭隐藏到托盘，服务器继续运行 | [ ] |
| 托盘菜单显示 Gateway 状态和端口 | [ ] |
| 托盘退出时服务器优雅关闭，无残留进程 | [ ] |
| 托盘菜单可打开配置/日志目录 | [ ] |
| Debug 模式依赖外部 gateway | [ ] |
| CLI 命令 `agent-diva gateway run` 继续独立运行 | [ ] |
| 多次启动无端口冲突、无孤儿进程 | [ ] |
| Windows 平台测试通过 | [ ] |
| Linux 平台测试通过（如条件允许） | [ ] |
| process_utils.rs 标记 deprecated | [ ] |
| 文档更新完成 | [ ] |

---

## 4. 回归测试脚本

```bash
# 完整回归测试脚本
#!/bin/bash

echo "=== Phase 4 Regression Test ==="

# 1. 编译检查
echo "1. Building..."
cargo build -p agent-diva-gui --release || exit 1
cargo clippy -p agent-diva-gui -- -D warnings || exit 1

# 2. Release 启动测试
echo "2. Testing release startup..."
cargo run -p agent-diva-gui --release &
GUI_PID=$!
sleep 5

# 3. 端口检查
echo "3. Checking port file..."
PORT=$(cat ~/.agent-diva/gateway.port)
echo "Port: $PORT"

curl -s http://127.0.0.1:$PORT/api/health || exit 1

# 4. 托盘测试（手动）
echo "4. Manual tray test required..."

# 5. 清理
echo "5. Cleanup..."
kill $GUI_PID 2>/dev/null
sleep 2

# 6. 进程残留检查
echo "6. Checking residual processes..."
if pgrep -f "agent-diva" > /dev/null; then
    echo "ERROR: Residual process found"
    pkill -9 -f "agent-diva"
    exit 1
fi

echo "=== All tests passed ==="
```

---

## 5. 迭代日志记录

按照 AGENTS.md 规则 `iteration-log-required`，创建迭代记录：

**目录**：`docs/logs/2026-04-gateway-embedded-upgrade/v1.0.0-embedded-gateway/`

**文件**：
- `summary.md`: 改造完成总结
- `verification.md`: 测试验证记录
- `release.md`: 发布说明（或说明不适用）
- `acceptance.md`: 用户验收步骤

---

## 6. 下一步（可选增强）

完成 Phase 4 后，可选功能增强：

| 功能 | 描述 | 优先级 |
|------|------|--------|
| TUI 日志模式 | 统一日志输出到 TUI 界面 | 低 |
| 自动更新 | 参考 openfang updater.rs | 低 |
| 状态图标 | 托盘图标随状态变化 | 中 |
| 崩溃恢复 | Gateway 崩溃后自动重启 | 中 |

---

## 7. 参考文档

- [overview.md](./overview.md) - 总体架构
- [phase1.md](./phase1.md) - 基础设施改造
- [phase2.md](./phase2.md) - 生命周期整合
- [phase3.md](./phase3.md) - 托盘增强
- [reference.md](./reference.md) - 参考架构对比
- [../../logs/2026-04-gateway-embedded-upgrade/prd.md](../../logs/2026-04-gateway-embedded-upgrade/prd.md) - PRD 文档