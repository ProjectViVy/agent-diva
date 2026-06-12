# Agent Diva 单体安装包应用技术路线调研

> 调研日期：2026-03-02
> 项目：Agent Diva - 模块化 AI 助手框架

## 一、项目现状分析

### 1.1 当前架构

Agent Diva 是一个 Rust Cargo workspace 项目，包含以下组件：

| Crate | 功能 | 依赖关系 |
|-------|------|----------|
| `agent-diva-core` | 核心基础（消息总线、配置、会话管理） | 所有其他 crate |
| `agent-diva-agent` | Agent 循环、上下文构建、技能加载 | core |
| `agent-diva-providers` | LLM 提供商接口与实现 | core |
| `agent-diva-channels` | 聊天平台频道处理器 | core |
| `agent-diva-tools` | 工具系统（文件系统、Shell、Web 等） | core |
| `agent-diva-cli` | CLI 入口点 | agent, manager |
| `agent-diva-migration` | Python 版本迁移工具 | core |
| `agent-diva-gui` | Tauri 桌面 GUI | 外部独立 crate |
| `agent-diva-manager` | API 管理服务器 | core, agent, providers, channels |

### 1.2 现有 GUI 基础

项目已包含 `agent-diva-gui` crate，使用 Tauri 2 框架：

```json
// tauri.conf.json
{
  "productName": "agent-diva-gui",
  "identifier": "com.com01.agent-diva-gui",
  "bundle": {
    "active": true,
    "targets": "all"
  }
}
```

前端技术栈：Vue.js + Vite + TailwindCSS

---

## 二、核心架构设计：守护进程模式

### 2.1 为什么需要守护进程？

Agent Diva 是一个**常驻型** AI 助手服务，具有以下特征：

| 需求 | 说明 |
|------|------|
| **持续监听** | 需要持续监听 Telegram、Discord、Slack 等 9 个聊天平台的消息 |
| **长连接维护** | 保持与 LLM 提供商的 WebSocket/HTTP 连接 |
| **状态管理** | 维护会话状态、长期记忆（MEMORY.md/HISTORY.md） |
| **定时任务** | 处理 cron 定时任务和调度 |
| **自动重启** | 崩溃后自动恢复，保持服务可用性 |

因此，Agent Diva **必须**作为系统服务/守护进程运行，而非一次性命令行工具。

### 2.2 推荐架构：控制面板 + 守护进程

```
┌─────────────────────────────────────────────────────────────┐
│                    用户交互层                                │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────────────┐      ┌──────────────────────┐   │
│  │   Tauri GUI 窗口      │      │     CLI 工具          │   │
│  │   (控制面板)          │      │  (高级用户/服务器)    │   │
│  ├──────────────────────┤      ├──────────────────────┤   │
│  │ • 服务状态监控        │      │ • agent-diva status  │   │
│  │ • 配置编辑器          │      │ • agent-diva start   │   │
│  │ • 日志查看器          │      │ • agent-diva stop    │   │
│  │ • 会话管理            │      │ • agent-diva logs    │   │
│  │ • 技能管理            │      │ • agent-diva config  │   │
│  └──────────────────────┘      └──────────────────────┘   │
│             │                            │                 │
│             └────────────┬───────────────┘                 │
│                          ▼                                 │
├─────────────────────────────────────────────────────────────┤
│                    IPC 通信层                                │
│                    (HTTP API / Unix Socket)                 │
├─────────────────────────────────────────────────────────────┤
│                          │                                 │
└──────────────────────────┼─────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────┐
│              Agent Diva 守护进程 (系统服务)                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │            Gateway 服务 (agent-diva gateway)         │   │
│  ├─────────────────────────────────────────────────────┤   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │   │
│  │  │  Channels   │  │   Agent     │  │   Tools     │  │   │
│  │  │  • Telegram │  │  • Loop     │  │  • Shell    │  │   │
│  │  │  • Discord  │  │  • Context  │  │  • FS       │  │   │
│  │  │  • Slack    │  │  • Memory   │  │  • Web      │  │   │
│  │  │  • WhatsApp │  │  • Skills   │  │  • Cron     │  │   │
│  │  │  • ...x5    │  │             │  │             │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │         Manager API (agent-diva manager)             │   │
│  │         • REST API for remote control               │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    持久化存储                                  │
├─────────────────────────────────────────────────────────────┤
│  ~/.agent-diva/                                             │
│  ├── config.json          (配置文件)                         │
│  ├── sessions/            (会话持久化 JSONL)                │
│  ├── MEMORY.md            (长期记忆)                         │
│  ├── HISTORY.md           (历史记录)                         │
│  ├── skills/              (用户技能)                         │
│  └── logs/                (运行日志)                         │
└─────────────────────────────────────────────────────────────┘
```

---

## 三、跨平台服务实现方案

### 3.1 各平台服务化方案

| 平台 | 服务机制 | 实现方式 | 自动启动 |
|------|----------|----------|----------|
| **Windows** | Windows Service | `windows-service` crate | ✅ |
| **Linux** | systemd | systemd unit 文件 | ✅ |
| **macOS** | launchd | LaunchAgent plist | ✅ |

### 3.2 Windows 服务实现

**依赖 crate:**

```toml
[dependencies]
windows-service = "0.7"
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_System_Services",
    "Win32_Security",
]}
```

**实现示例:**

```rust
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState,
        ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher, Result,
};

define_windows_service!(ffi_service_main, service_main);

fn service_main(arguments: Vec<String>) {
    if let Err(_e) = run_service(arguments) {
        // Handle error
    }
}

fn run_service(arguments: Vec<String>) -> Result<()> {
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                // Graceful shutdown
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(
        "AgentDivaGateway",
        event_handler,
    )?;

    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        ..Default::default()
    };
    status_handle.set_service_status(next_status)?;

    // 运行 gateway 主逻辑
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            agent_diva_gateway::run().await;
        });

    Ok(())
}
```

**服务注册 (CLI 命令):**

```rust
use windows_service::{
    service::ServiceAccess,
    service_manager::{ServiceManager, ServiceManagerAccess},
};

fn install_service() -> Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_binary_path = std::env::current_exe()?;

    let service_info = windows_service::service::ServiceInfo {
        name: "AgentDivaGateway".into(),
        display_name: "Agent Diva Gateway Service".into(),
        service_type: ServiceType::OWN_PROCESS,
        start_type: windows_service::service::ServiceStartType::AutoStart,
        ..Default::default()
    };

    let _service = service_manager.create_service(
        &service_info,
        ServiceAccess::CHANGE_CONFIG,
        service_binary_path,
    )?;

    Ok(())
}
```

### 3.3 Linux systemd 服务

**Unit 文件安装路径:** `/etc/systemd/system/agent-diva.service`

```ini
[Unit]
Description=Agent Diva Gateway - AI Assistant Service
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=agent-diva
Group=agent-diva
ExecStart=/usr/bin/agent-diva gateway
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=5s

# 资源限制
LimitNOFILE=65536
LimitNPROC=4096

# 安全加固
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/agent-diva /var/log/agent-diva

# 环境变量
Environment="RUST_LOG=info"
Environment="AGENT_DIVA_CONFIG_DIR=/etc/agent-diva"

[Install]
WantedBy=multi-user.target
```

**服务管理命令:**

```bash
# 用户安装时执行
sudo systemctl daemon-reload
sudo systemctl enable agent-diva
sudo systemctl start agent-diva

# 查看状态
sudo systemctl status agent-diva

# 查看日志
sudo journalctl -u agent-diva -f
```

### 3.4 macOS LaunchAgent

**Plist 文件:** `~/Library/LaunchAgents/com.agent-diva.gateway.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.agent-diva.gateway</string>

    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/agent-diva</string>
        <string>gateway</string>
    </array>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
        <key>Crashed</key>
        <true/>
    </dict>

    <key>StandardOutPath</key>
    <string>/var/log/agent-diva/gateway.log</string>

    <key>StandardErrorPath</key>
    <string>/var/log/agent-diva/gateway.error.log</string>

    <key>EnvironmentVariables</key>
    <dict>
        <key>RUST_LOG</key>
        <string>info</string>
    </dict>

    <key>WorkingDirectory</key>
    <string>/var/lib/agent-diva</string>
</dict>
</plist>
```

**服务管理命令:**

```bash
# 加载服务
launchctl load ~/Library/LaunchAgents/com.agent-diva.gateway.plist

# 启动服务
launchctl start com.agent-diva.gateway

# 停止服务
launchctl stop com.agent-diva.gateway

# 查看状态
launchctl list | grep agent-diva
```

---

## 四、技术路线方案

### 路线 A：Tauri 桌面应用（推荐）

#### 架构概述

```
┌─────────────────────────────────────────┐
│           Tauri 桌面应用窗口              │
├─────────────────────────────────────────┤
│  前端: Vue.js + Vite + TailwindCSS       │
│  ├─ 配置管理界面                         │
│  ├─ 日志查看器                           │
│  ├─ 服务状态监控                         │
│  └─ 交互式终端                           │
├─────────────────────────────────────────┤
│  后端: Tauri Commands (Rust)            │
│  ├─ 与守护进程通信 (HTTP/Unix Socket)    │
│  ├─ 管理 Agent 配置                      │
│  ├─ 实时日志流                           │
│  └─ 服务安装/卸载                        │
└─────────────────────────────────────────┘
                    ↕ (IPC)
┌─────────────────────────────────────────┐
│      Agent Diva 守护进程 (系统服务)       │
│                                         │
│  ┌─────────────────────────────────┐    │
│  │  Gateway (Channels+Agent+Tools) │    │
│  │  • 持续监听 9 个聊天平台         │    │
│  │  • 处理 LLM 调用                │    │
│  │  • 执行工具调用                 │    │
│  └─────────────────────────────────┘    │
│                                         │
│  ┌─────────────────────────────────┐    │
│  │  Manager API                    │    │
│  │  • RESTful 管理接口             │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

#### 实现步骤

1. **整合所有 crate 到 Tauri 后端**
   ```toml
   # agent-diva-gui/src-tauri/Cargo.toml
   [dependencies]
   agent-diva-core = { path = "../../agent-diva-core" }
   agent-diva-agent = { path = "../../agent-diva-agent" }
   agent-diva-providers = { path = "../../agent-diva-providers" }
   agent-diva-channels = { path = "../../agent-diva-channels" }
   agent-diva-tools = { path = "../../agent-diva-tools" }
   ```

2. **Tauri Commands 实现**
   ```rust
   #[tauri::command]
   async fn start_gateway(config: GatewayConfig) -> Result<String, String> {
       // 启动 gateway 服务
   }

   #[tauri::command]
   async fn get_logs(lines: usize) -> Result<Vec<String>, String> {
       // 读取日志
   }

   #[tauri::command]
   fn get_config() -> Result<Config, String> {
       // 获取当前配置
   }
   ```

3. **资源嵌入（可选）**
   - 使用 `include_dir` crate 嵌入默认技能文件
   - 嵌入默认配置模板

4. **打包配置**
   ```json
   {
     "bundle": {
       "active": true,
       "targets": ["msi", "nsis", "app", "dmg", "deb", "appimage"],
       "icon": ["icons/*.png", "icons/*.ico", "icons/*.icns"]
     }
   }
   ```

#### 优势

| 特性 | 说明 |
|------|------|
| 体积小 | 打包后 < 3MB（不含前端资源） |
| 性能优 | 冷启动 ~300ms |
| 安全性高 | Rust 后端，无 Chromium |
| 跨平台 | Windows、macOS、Linux 一键打包 |
| 成熟度高 | 项目已有基础，仅需增强 |

#### 打包命令

```bash
cd agent-diva-gui/src-tauri
# 开发模式
cargo tauri dev

# 生产构建
cargo tauri build

# 指定平台
cargo tauri build --target universal-apple-darwin  # macOS
cargo tauri build --target x86_64-pc-windows-msvc  # Windows
```

---

### 路线 B：纯 CLI + 安装器方案

#### 架构概述

```
┌─────────────────────────────────────┐
│      传统安装包 (MSI/DEB/RPM)        │
├─────────────────────────────────────┤
│  安装内容:                           │
│  ├─ agent-diva 可执行文件            │
│  ├─ 系统服务注册 (scsd/systemd)      │
│  ├─ 配置文件模板                     │
│  ├─ 默认技能文件                     │
│  └─ 文档/快捷方式                    │
└─────────────────────────────────────┘
         ↓
┌─────────────────────────────────────┐
│      Windows 服务 / Linux Daemon     │
├─────────────────────────────────────┤
│  后台运行 Gateway                    │
│  监听配置变化自动重启                 │
└─────────────────────────────────────┘
```

#### 实现工具

| 平台 | 工具 | 说明 |
|------|------|------|
| Windows | WiX Toolset / NSIS | MSI/EXE 安装器 |
| macOS | Packages / .dmg | DMG 镜像 + PKG |
| Linux | .deb / .rpm | 系统原生包格式 |

#### 优势

- 服务器/无头环境友好
- 符合系统管理规范
- 可作为系统服务运行

#### 劣势

- 缺少图形界面
- 配置管理相对复杂

---

### 路线 C：Dioxus 原生 Rust GUI（备选）

#### 架构概述

Dioxus 是纯 Rust 的 GUI 框架，可生成真正的单文件可执行程序。

```
┌─────────────────────────────────────┐
│         Dioxus Desktop App           │
├─────────────────────────────────────┤
│  UI: RSX (Rust JSX)                 │
│  ├─ 跨平台原生渲染                   │
│  └─ 无 WebView 依赖                  │
├─────────────────────────────────────┤
│  Logic: 所有业务逻辑内嵌              │
│  └─ 无需子进程通信                   │
└─────────────────────────────────────┘
```

#### 优势

- 真正的单文件可执行程序
- 无前端构建依赖
- 完全 Rust 技术栈

#### 劣势

- 生态相对较小
- UI 开发效率不如 Web 技术栈
- 学习曲线较陡

---

### 路线 D：嵌入式单二进制方案

#### 架构概述

使用 `include_dir` crate 将所有资源嵌入单个可执行文件。

```rust
use include_dir::{include_dir, Dir};

static SKILLS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/skills");
static CONFIG_TEMPLATE: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/config-templates");

// 运行时解压到用户目录
fn init_assets() -> Result<()> {
    let base_dir = dirs::home_dir()?.join(".agent-diva");
    SKILLS_DIR.extract(&base_dir.join("skills"))?;
    CONFIG_TEMPLATE.extract(&base_dir)?;
    Ok(())
}
```

#### 适用场景

- 便携式应用（U盘运行）
- 无需安装的绿色软件
- 最小化依赖场景

---

## 三、技术对比矩阵

| 特性 | Tauri | CLI+Installer | Dioxus | 单二进制 |
|------|-------|---------------|--------|----------|
| **包体积** | ~3MB | ~2MB | ~5MB | ~10MB |
| **启动速度** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **开发效率** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ |
| **用户体验** | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ |
| **跨平台** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **维护成本** | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| **生态支持** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |

---

## 四、推荐方案：混合路线

结合 Tauri 桌面应用 + CLI 工具，提供灵活的部署选项：

```
agent-diva/
├── agent-diva-gui/          # 桌面应用包 (推荐给普通用户)
│   ├── src-tauri/           # Tauri 后端
│   └── src/                 # Vue 前端
│
├── agent-diva-cli/          # CLI 工具 (推荐给高级用户/服务器)
│
└── distrib/                 # 分发包
    ├── agent-diva-setup.exe # Windows 安装器
    ├── agent-diva.dmg       # macOS 安装器
    └── agent-diva.deb       # Linux 包
```

### 发布产物

1. **桌面用户**：下载 GUI 安装包，一键安装，图形化管理
2. **服务器用户**：下载 CLI 单文件，配置服务运行
3. **开发者**：Cargo 安装或源码编译

---

## 五、实施路线图

### 阶段 1：Tauri GUI 增强（2周）

- [ ] 整合所有 crate 到 Tauri 后端
- [ ] 实现核心 Commands（启动/停止/配置）
- [ ] 添加实时日志流功能
- [ ] 实现配置编辑器

### 阶段 2：打包配置（1周）

- [ ] 配置多平台打包目标
- [ ] 设计应用图标和品牌
- [ ] 配置代码签名（macOS/Windows）

### 阶段 3：安装器开发（1周）

- [ ] Windows MSI/NSIS 配置
- [ ] macOS DMG 制作
- [ ] Linux .deb/.rpm 构建

### 阶段 4：测试与发布（1周）

- [ ] 多平台安装测试
- [ ] 自动更新机制（可选）
- [ ] 文档编写

---

## 六、资源需求

### 开发环境

| 工具 | 用途 |
|------|------|
| Rust 1.75+ | 核心开发 |
| Node.js 18+ | 前端构建 |
| pnpm | 前端包管理 |
| WiX Toolset | Windows 打包 |
| GNU tar | macOS 打包 |
| rpm/deb-tools | Linux 打包 |

### CI/CD

```yaml
# GitHub Actions 示例
build:
  strategy:
    matrix:
      platform: [windows-latest, macos-latest, ubuntu-latest]
  steps:
    - uses: actions/checkout@v3
    - uses: actions-rust-lang/setup-rust-toolchain@v1
    - run: cd agent-diva-gui/src-tauri && cargo tauri build
```

---

## 七、参考资源

### 技术文档

- [Tauri 2 官方文档 - 分发与打包](https://v2.tauri.app/distribute/)
- [Tauri Windows 安装器指南](https://v2.tauri.app/distribute/windows-installer/)
- [Dioxus 部署指南](https://dioxuslabs.com/learn/0.6/guide/deploy/)
- [include_dir 文档](https://docs.rs/include_dir/latest/include_dir/)

### 对比分析

- [Tauri vs Electron 2025 对比](https://lobehub.com/skills/bobmatnyc-claude-mpm-desktop-applications)

### 社区资源

- [Rust 跨平台打包讨论](https://internals.rust-lang.org/t/cross-platform-bundling/16773)
- [Cargo Workspaces 官方文档](https://doc.rust-lang.org/cargo/reference/workspaces.html)

---

## 八、结论

**推荐采用 Tauri 路线**，理由如下：

1. **项目已有基础**：`agent-diva-gui` 已存在，仅需增强功能
2. **用户体验优秀**：图形化界面降低使用门槛
3. **技术成熟度高**：Tauri 2 已稳定，文档完善
4. **打包体积小**：符合单体应用分发需求
5. **社区活跃**：问题解决和技术支持便利

对于服务器/无头环境，保留 CLI 工具作为补充方案。
