# Phase 3：托盘增强与体验优化

> 目标：扩展托盘菜单功能，增加 Gateway 状态显示和目录快捷打开

---

## 1. 步骤概览

| 步骤 | 文件 | 操作 |
|------|------|------|
| 3.1 | `agent-diva-gui/src-tauri/src/tray.rs` | 扩展托盘菜单 |
| 3.2 | `agent-diva-gui/src-tauri/src/tray.rs` | 实现目录打开功能 |
| 3.3 | `agent-diva-gui/src-tauri/src/tray.rs` | 动态状态更新 |
| 3.4 | `agent-diva-gui/src-tauri/src/lib.rs` | Splash screen 就绪检测简化 |
| 3.5 | `agent-diva-gui/src-tauri/Cargo.toml` | 新增 open crate 依赖 |
| 3.6 | 测试验证 | 托盘功能验证 |

---

## 2. 详细步骤

### 2.1 托盘菜单扩展

**文件**：`agent-diva-gui/src-tauri/src/tray.rs`

#### 2.1.1 当前菜单结构

```rust
// 当前仅有两项
let show_item = MenuItem::with_id(app, "show", "Show Window")?;
let quit_item = MenuItem::with_id(app, "quit", "Quit")?;
let menu = Menu::with_items(app, &[&show_item, &quit_item])?;
```

#### 2.1.2 扩展菜单结构

```rust
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};

fn build_tray_menu<R: Runtime>(app: &AppHandle<R>) -> Result<Menu<R>, Box<dyn std::error::Error>> {
    // 显示窗口
    let show_item = MenuItem::with_id(app, "show", "Show Window")?;
    
    // Gateway 状态（动态更新，禁用状态）
    let status_item = MenuItem::with_id(app, "gateway_status", "Gateway: Running (port: ---)")?;
    
    // 分隔线
    let separator = PredefinedMenuItem::separator(app)?;
    
    // 打开目录
    let config_item = MenuItem::with_id(app, "open_config", "Open Config Directory")?;
    let logs_item = MenuItem::with_id(app, "open_logs", "Open Logs Directory")?;
    
    // 分隔线
    let separator2 = PredefinedMenuItem::separator(app)?;
    
    // 退出
    let quit_item = MenuItem::with_id(app, "quit", "Quit")?;
    
    Menu::with_items(app, &[
        &show_item,
        &separator,
        &status_item,
        &config_item,
        &logs_item,
        &separator2,
        &quit_item,
    ])?
}
```

#### 2.1.3 菜单结构示意

```
┌─────────────────────────────┐
│ Show Window                 │
│ ─────────────────────────── │
│ Gateway: Running (port: 52341) │  ← 禁用，仅显示
│ Open Config Directory       │
│ Open Logs Directory         │
│ ─────────────────────────── │
│ Quit                        │
└─────────────────────────────┘
```

---

### 2.2 目录打开功能

**文件**：`agent-diva-gui/src-tauri/src/tray.rs`

#### 2.2.1 新增依赖

**文件**：`agent-diva-gui/src-tauri/Cargo.toml`

```toml
[dependencies]
open = "5"  # 跨平台目录/文件打开
```

#### 2.2.2 实现目录打开函数

```rust
use open;

/// Open config directory in system file explorer.
fn open_config_directory() {
    let config_dir = get_config_directory();
    if config_dir.exists() {
        if let Err(e) = open::that(&config_dir) {
            tracing::error!("Failed to open config directory: {}", e);
        }
    } else {
        tracing::warn!("Config directory does not exist: {}", config_dir.display());
    }
}

/// Open logs directory in system file explorer.
fn open_logs_directory() {
    let logs_dir = get_config_directory().join("logs");
    if logs_dir.exists() {
        if let Err(e) = open::that(&logs_dir) {
            tracing::error!("Failed to open logs directory: {}", e);
        }
    } else {
        tracing::warn!("Logs directory does not exist: {}", logs_dir.display());
    }
}

/// Get config directory path.
fn get_config_directory() -> std::path::PathBuf {
    // 使用 ConfigLoader 的默认路径
    dirs::data_local_dir()
        .unwrap_or_default()
        .join(".agent-diva")
}
```

---

### 2.3 动态状态更新

**文件**：`agent-diva-gui/src-tauri/src/tray.rs`

#### 2.3.1 菜单事件处理

```rust
fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: &MenuEvent) {
    match event.id().as_ref() {
        "show" => {
            show_main_window(app);
        }
        "open_config" => {
            open_config_directory();
        }
        "open_logs" => {
            open_logs_directory();
        }
        "quit" => {
            tracing::info!("Quit requested from system tray");
            // 触发 shutdown（通过 window close 或直接调用）
            app.exit(0);
        }
        _ => {}
    }
}
```

#### 2.3.2 状态更新函数

```rust
use crate::gateway_status::GatewayStatus;

/// Update tray menu status text with current gateway status.
pub fn update_tray_status<R: Runtime>(app: &AppHandle<R>) {
    // 获取 GatewayStatus from managed state
    let status = app.state::<GatewayStatus>();
    let status_text = status.format_status();
    
    // 更新菜单项文本
    if let Ok(menu) = app.tray().menu() {
        if let Ok(items) = menu.items() {
            for item in items {
                if item.id() == "gateway_status" {
                    // MenuItem::set_text 在 Tauri 2.x 中可用
                    if let Ok(menu_item) = item.try_downcast::<MenuItem<R>>() {
                        menu_item.set_text(status_text).ok();
                    }
                }
            }
        }
    }
}
```

#### 2.3.3 初始化时设置状态

在 `init_tray` 函数中：

```rust
pub fn init_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 构建菜单
    let menu = build_tray_menu(app)?;
    
    // 2. 创建托盘图标
    let tray = TrayIconBuilder::new()
        .show_menu_on_left_click(false)
        .menu(&menu)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_tray_icon_event)
        .build(app)?;
    
    // 3. 初始化状态显示
    update_tray_status(app);
    
    Ok(())
}
```

---

### 2.4 Splash Screen 就绪检测简化

**文件**：`agent-diva-gui/src-tauri/src/lib.rs`

#### 2.4.1 当前实现

```rust
// 当前等待 frontend + backend 各 500ms
// backend_done 在子进程启动后设置
```

#### 2.4.2 简化方案

内嵌模式下端口在 setup 阶段已确定，无需等待外部进程：

```rust
// setup hook 中，内嵌启动成功后立即设置 backend_done
if should_manage_gateway_lifecycle() {
    let handle = embedded_server::start_embedded_gateway(config)?;
    
    // 端口已确定，服务器后台启动中
    // 设置 backend_done，splash 可以关闭
    splash_state.set_backend_done();
}
```

#### 2.4.3 可选增强：内部健康检查

如果需要确保服务器完全就绪再关闭 splash：

```rust
// 在 embedded_server.rs 中添加 ready 信号
pub struct EmbeddedGatewayHandle {
    pub port: u16,
    ready_rx: oneshot::Receiver<bool>,  // 新增
    // ...
}

// setup hook 中等待 ready
if let Ok(true) = handle.ready_rx.await {
    splash_state.set_backend_done();
}
```

---

### 2.5 托盘图标状态指示（可选增强）

可扩展为根据 Gateway 状态变化图标：

```rust
pub fn update_tray_icon<R: Runtime>(app: &AppHandle<R>, running: bool) {
    if let Ok(tray) = app.tray() {
        let icon_path = if running {
            "icons/icon.png"      // 正常图标
        } else {
            "icons/icon-stopped.png"  // 停止图标（需准备）
        };
        tray.set_icon(tauri::image::Image::from_path(icon_path).ok()).ok();
    }
}
```

---

## 3. 测试验证

### 3.1 托盘菜单验证

**验证项**：

| 功能 | 验证方法 | 预期结果 |
|------|----------|----------|
| 菜单结构 | 右键托盘图标 | 显示 6 项菜单 |
| 状态显示 | 查看菜单项 | 显示 "Gateway: Running (port: xxx)" |
| Show Window | 点击菜单项 | 窗口显示/恢复 |
| Open Config | 点击菜单项 | 打开 .agent-diva 目录 |
| Open Logs | 点击菜单项 | 打开 logs 子目录 |
| Quit | 点击菜单项 | 触发 shutdown，进程退出 |

### 3.2 目录打开验证

```bash
# 确认目录存在
ls ~/.agent-diva
ls ~/.agent-diva/logs

# 点击菜单后确认打开
# Windows: 打开 Explorer
# Linux: 打开默认文件管理器
```

### 3.3 Splash Screen 验证

**Release 模式**：
- 启动 GUI → splash 显示 → 端口确定后 splash 关闭
- 观察启动时间是否缩短（移除 500ms 延迟）

---

## 4. 关键依赖

```
tray.rs
  └── tauri::menu::{Menu, MenuItem, PredefinedMenuItem}
  └── open::that(path)  # 跨平台打开
  └── GatewayStatus (managed state)
  
Cargo.toml
  └── open = "5"
```

---

## 5. 跨平台注意事项

| 平台 | 注意事项 |
|------|----------|
| **Windows** | `open::that()` 使用 `explorer.exe`，可能被 Defender 拦截 |
| **Linux** | 使用 `xdg-open`，需确保桌面环境支持 |
| **macOS** | 使用 `open` 命令，通常无问题 |

---

## 6. 潜在问题与解决方案

| 问题 | 解决方案 |
|------|----------|
| MenuItem::set_text Tauri API 版本 | 确认 Tauri 2.x 支持，或使用 rebuild menu 方案 |
| 目录不存在时的错误处理 | 显示 warning 日志，不 crash |
| 状态更新时机 | 在 shutdown 时调用 update_tray_status 设置 "Stopped" |

---

## 7. 下一步

Phase 3 完成后，进入 [Phase 4：清理与验证](./phase4.md)，完成最终清理和完整测试。