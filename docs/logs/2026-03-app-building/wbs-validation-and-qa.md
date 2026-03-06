---
title: Agent Diva 跨平台构建验证与 QA WBS
---

> 使用说明（面向 Agent）：  
> 当你（Agent 或子 Agent）负责“验证与 QA”时，本文件告诉你在不同平台/模式下应该执行哪些 smoke 与回归检查。  
> 每个 WP 中的步骤都可以由你自动化执行（例如在 CI Job 或专用测试环境中），并把结果写回日志或报告。

## 1. 控制账户（CA）概览

- **CA-QA-SMOKE-DESKTOP：桌面 GUI smoke 测试**
  - 目标：在三平台上验证“安装 → 启动 → 关键页面可用”的最小闭环。

- **CA-QA-SMOKE-HEADLESS：Headless 服务 smoke 测试**
  - 目标：验证 CLI/服务形态在服务器环境中的最小运行与健康检查。

- **CA-QA-REGRESSION：基础回归与规则校验**
  - 目标：确保每次构建都执行 workspace 级基础校验，与仓库规则保持一致。

---

## 2. CA-QA-SMOKE-DESKTOP：桌面 GUI smoke 测试

### CI 工件映射（桌面 GUI）

| 平台 | CI artifact 名称模式 | 主要安装包位置 | 对应分发 WP |
| --- | --- | --- | --- |
| Windows | `agent-diva-gui-windows-<arch>-<sha>` | `bundle/nsis/`、`bundle/msi/` | `WP-DIST-GUI-01`、`WP-DIST-GUI-02` |
| macOS | `agent-diva-gui-macos-<arch>-<sha>` | `bundle/dmg/`、`bundle/macos/` | `WP-DIST-GUI-01`、`WP-DIST-GUI-03` |
| Linux | `agent-diva-gui-linux-<arch>-<sha>` | `bundle/deb/`、`bundle/appimage/` | `WP-DIST-GUI-01`、`WP-DIST-GUI-04` |

执行 smoke 前，先从 CI 下载对应 artifact，再进入平台对应的 `bundle/` 子目录选择安装包。

### WP-QA-DESKTOP-01：Windows GUI smoke

- **概述**
  - 验证 Windows 安装器与 GUI 主功能路径的基本可用性。

- **先决条件**
  - 已有 Windows 安装包（NSIS/MSI）；
  - 该安装包来自 `agent-diva-gui-windows-<arch>-<sha>` artifact；
  - 具有可重置的 Windows 测试环境（物理机或 VM）。

- **实施步骤**
  1. 在干净环境中运行安装包，记录安装路径（默认为 `C:\Program Files\AgentDiva\` 或文档指定目录）。
  2. 从开始菜单或桌面快捷方式启动 `Agent Diva`：
     - 确认主窗口正常显示；
     - 仪表盘页面可加载且无致命报错。
  3. 在 GUI 内执行以下操作：
     - 打开“中控台”，验证可以看到 gateway 进程状态、Manager 健康状态、原始配置编辑器与日志面板；
     - 点击“重新载入”读取原始配置，确认 JSON 内容可以正常显示；
     - 打开配置页面，验证能加载配置内容；
     - 打开日志页面，即便无日志也有明确“暂无日志”提示；
     - **服务管理面板**：打开“设置 → 通用”，验证 ServiceManagementPanel 区域可见；打包应用中应显示服务状态卡片（installed/running）、刷新按钮及安装/启动/停止/卸载操作按钮；开发模式下应显示“仅打包应用可用”占位。
  4. 关闭 GUI，并从“添加或删除程序”中卸载应用：
     - 确认应用目录被删除；
     - 用户数据目录（如 `C:\Users\<User>\.agent-diva`）按设计保留。
  5. 如本次构建启用了服务安装选项，再执行一轮可选验证：
     - 勾选安装器中的 “Install and start Agent Diva Gateway as a Windows Service”；
     - 若构建已附带 `agent-diva-service.exe`，验证 `services.msc` 中出现 `AgentDivaGateway`；
     - 若未附带，则应收到清晰提示，说明该能力尚未随当前安装包启用。

- **测试与验收**
  - 上述步骤无崩溃或严重异常；
  - 若有错误提示，需可读且指向明确的解决路径。
  - 需在验证记录中回填：
    - artifact 名称
    - 安装包文件名
    - 是否执行了服务安装分支
    - `WP-DIST-GUI-02` 的执行结论

---

### WP-QA-DESKTOP-02：macOS GUI smoke

- **概述**
  - 验证 macOS dmg/app 安装路径与基础使用流程。

- **先决条件**
  - 已有 macOS `.dmg` 安装包；
  - 该安装包来自 `agent-diva-gui-macos-<arch>-<sha>` artifact；
  - 有一台支持目标 macOS 版本的设备或 VM。

- **实施步骤**
  1. 双击 `.dmg`，将 `Agent Diva.app` 拖入 `Applications`。
  2. 首次启动应用：
     - 若有 Gatekeeper 提示，按文档指引允许运行；
     - 确认主界面加载正常。
  3. 重复 Windows smoke 步骤中的 GUI 操作（中控台 / 配置 / 日志页面），并打开“设置 → 通用”验证 **服务管理面板**：打包应用中应显示服务状态与操作按钮；macOS 上应显示受控降级提示（`servicePlatformPending`）。
  4. 删除 `Applications` 中的 `Agent Diva.app`，确认用户数据保留在 `~/.agent-diva`。
  5. 在记录中标注本轮使用的是：
     - `.dmg` 安装路径
     - 还是直接从 `.app` 启动的开发/验收路径

- **测试与验收**
  - 主窗口无渲染问题（字体/布局问题仅作为 UI bug 记录）；
  - 日志中无高频 panic 或未捕获异常。
  - 验收记录需显式引用 `WP-DIST-GUI-03`，确认 `.app` 与 `~/.agent-diva` 的目录约定一致。

---

### WP-QA-DESKTOP-03：Linux GUI smoke

- **概述**
  - 验证 Linux 上 deb/appimage 安装路径与基础 GUI 使用流程。

- **先决条件**
  - 已有 `.deb` 包和/或 `.AppImage`；
  - 该安装包来自 `agent-diva-gui-linux-<arch>-<sha>` artifact；
  - 目标发行版（如 Ubuntu LTS）的 VM 环境。

- **实施步骤**
  1. 使用 `sudo dpkg -i xxx.deb` 安装（如有依赖问题，通过 `apt -f install` 解决），或直接 `chmod +x xxx.AppImage && ./xxx.AppImage` 运行。
  2. 启动应用并执行与 Windows/macOS smoke 相同的 GUI 操作，至少覆盖：
     - 打开“中控台”查看 gateway 状态、原始配置、日志面板；
     - 打开“设置 → 通用”验证 **服务管理面板**：应显示 systemd 服务状态卡片、刷新按钮及安装/启动/停止/卸载 systemd 服务按钮；状态与操作应符合 [ui-ca-gui-arch-service-management-panel.md](ui-ca-gui-arch-service-management-panel.md) 设计规范。
  3. 通过系统应用菜单卸载（deb 路径）时确认二进制与快捷方式删除情况。
  4. 至少覆盖两条路径中的一条：
     - `.deb` 原生安装/卸载
     - `.AppImage` 便携启动/删除

- **测试与验收**
  - GUI 能正常渲染并响应基本操作；
  - 无权限错误阻止配置/日志读取（如有需在文档中说明需额外设置）。
  - 验收记录需显式引用 `WP-DIST-GUI-04`，并写明采用的是 `.deb` 还是 `.AppImage` 路径。

---

## 3. CA-QA-SMOKE-HEADLESS：Headless 服务 smoke 测试

### WP-QA-HEADLESS-01：Linux systemd smoke

- **概述**
  - 验证在 Linux 上使用 systemd 管理 Agent Diva 服务的最小流程。
  - 对应 `CA-HL-LNX-SYSTEMD` 与 `wbs-headless-service-mode.md` 中的 WP-HL-LNX-01/02。

- **输入 artifact**
  - 来自 `CA-CI-ARTIFACTS` 或 `CA-CI-MATRIX` 的 Linux Headless 压缩包：`agent-diva-{version}-linux-{arch}.tar.gz`
  - 包内需包含：`bin/agent-diva`、`systemd/agent-diva.service`、`systemd/install.sh`、`systemd/uninstall.sh`

- **先决条件**
  - 已构建好的 Headless 压缩包与 systemd unit 模板；
  - 测试环境具备 `sudo` 权限；
  - 目标系统使用 systemd（如 Ubuntu LTS、Debian、CentOS 等）。

- **实施步骤**
  1. 解压 Headless 包并执行安装脚本：

     ```bash
     tar -xzf agent-diva-{version}-linux-{arch}.tar.gz
     cd agent-diva-{version}-linux-{arch}/systemd
     sudo ./install.sh
     ```

  2. 验证服务状态与日志：

     ```bash
     sudo systemctl status agent-diva
     journalctl -u agent-diva -f --no-pager | tail -20
     ```

  3. 使用健康检查端点验证（若项目已暴露）：

     ```bash
     curl -sf http://127.0.0.1:PORT/health
     ```

  4. 重启系统后再次验证服务是否自动启动。

- **测试矩阵（Linux headless 服务场景）**

| 场景 | 输入 | 期望观察点 | 对应 WP |
| --- | --- | --- | --- |
| 首次安装 | Headless 压缩包 | `systemctl status agent-diva` 为 active (running)；`journalctl -u agent-diva` 有网关启动日志 | WP-HL-LNX-02 |
| 重启后自启 | 已安装服务 | 重启后 `systemctl is-enabled agent-diva` 为 enabled；服务自动运行 | WP-HL-LNX-01 |
| 手动停止/启动 | 已安装服务 | `systemctl stop` / `systemctl start` 无报错；状态正确切换 | WP-HL-LNX-01 |
| 升级覆盖安装 | 新版本 Headless 包 | 解压新版本、重新执行 `install.sh` 后，服务指向新二进制；无残留旧进程 | WP-HL-LNX-02 |
| 卸载保留数据目录 | 已安装服务 | 执行 `uninstall.sh` 后，`/usr/bin/agent-diva` 与 unit 文件被删除；`/var/lib/agent-diva`、`/var/log/agent-diva` 保留 | WP-HL-LNX-02 |

- **测试与验收**
  - 服务状态为 active (running)，健康检查返回 200 且内容符合预期（若端点存在）；
  - 重启后服务仍然运行；
  - 卸载后 `systemctl list-unit-files | grep agent-diva` 无结果，数据目录按设计保留。

---

### WP-QA-HEADLESS-02：Windows Service smoke

- **概述**
  - 验证 Windows Service 安装、启动、停止与卸载的基本路径。

- **先决条件**
  - `agent-diva.exe service *` 子命令已实现；
  - 有一台可重置的 Windows 测试机。

- **实施步骤**
  1. 以管理员身份打开 PowerShell：

     ```powershell
     .\agent-diva.exe service install --auto-start
     .\agent-diva.exe service start
     ```

  2. 在 `services.msc` 中查看 `AgentDivaGateway` 状态，应为“正在运行”；
  3. 可选：通过任务管理器确认后台进程存在；
  4. 测试停止与卸载：

     ```powershell
     .\agent-diva.exe service stop
     .\agent-diva.exe service uninstall
     ```

- **测试与验收**
  - 安装/启动/停止/卸载命令均无未处理异常；
  - 卸载后服务从 `services.msc` 中消失。

---

## 4. CA-QA-REGRESSION：基础回归与规则校验

### WP-QA-REG-00：GUI 服务管理面板行为校验（跨平台）

- **概述**
  - 当你（Agent）修改与服务管理相关的逻辑（包括 Tauri commands、CLI service 子命令、Headless 安装脚本等）时，应执行一轮从 GUI 视角出发的“安装 → 检查 → 卸载”闭环校验，确保 GUI 显示状态与系统实际状态一致。

- **先决条件**
  - 目标平台上 GUI 与服务安装脚本已按其他 WBS 配置完成；
  - 对应平台的 Headless 服务 smoke（如 `WP-QA-HEADLESS-01` / `WP-QA-HEADLESS-02`）已通过。

- **实施步骤**
  1. 在目标平台安装最新 GUI 构建，并确保当前处于“未安装服务”状态（如必要先通过 CLI 卸载一次）：  
     - Windows：确认 `services.msc` 中无 `AgentDivaGateway`；  
     - Linux：`systemctl status agent-diva` 显示 unit 不存在或 inactive；  
     - macOS：`launchctl list | grep agent-diva` 无结果，且 Plist 文件不存在。
  2. 启动 GUI，进入“设置 → 通用设置 → 服务管理”面板：  
     - 确认初始状态显示为“未安装”；  
     - 点击“安装服务”，等待状态刷新为“已安装/运行中”；  
     - 关闭并重新打开设置页，确认状态持久；  
     - 点击“卸载服务”，状态回到“未安装”。
  3. 若变更同时涉及 `CA-GUI-CMDS` 的中控台能力，再进入“中控台”执行一次回归：  
     - 查看 gateway 进程状态与 Manager 健康状态；  
     - 打开原始配置编辑器并执行一次只读 reload；  
     - 打开日志面板并确认最近 N 行能够正常刷新。
  4. 同时在系统层再次验证：  
     - Windows：`services.msc` 或 `Get-Service AgentDivaGateway`；  
     - Linux：`systemctl status agent-diva`；  
     - macOS：`launchctl list | grep agent-diva` 与 Plist 文件存在性。

- **测试与验收**
  - GUI 显示的“已安装/未安装/运行中”等状态与系统真实状态一致；  
  - 安装/卸载过程中若出现错误，你看到的错误提示应清晰可读、指向权限或环境问题，而不是静默失败或崩溃；  
  - 当 PR/变更涉及服务管理面板或相关 commands 时，应在变更说明中引用本 WP（`WP-QA-REG-00`）并记录执行结论。

### WP-QA-REG-01：仓库规则对齐（fmt/check/test）

- **概述**
  - 每次构建前后执行统一的基础校验，保证结构化质量门槛。

- **先决条件**
  - `justfile` 已定义 `fmt-check`、`check`、`test` 等命令。

- **实施步骤**
  1. 本地提交前，开发者需运行：

     ```bash
     just fmt-check
     just check
     just test
     ```

  2. CI 层面在 `rust-check` job 中已固化同样命令（见 CI/CD WBS）。

- **测试与验收**
  - 任一命令失败时，PR 不应被合并；
  - 对于引入 GUI/服务新能力的改动，在 MR/PR 描述中补充对应 smoke 测试说明（链接到本 WBS 中的具体 WP）。

---

### WP-QA-REG-02：变更影响范围标注与回归策略

- **概述**
  - 要求在每次涉及 GUI/Headless/安装器变更的提交中，明确列出影响范围与推荐回归路径。

- **先决条件**
  - 团队在 PR 模板或变更说明中预留相应字段。

- **实施步骤**
  1. 在 PR 模板中增加：
     - “影响范围”字段（如：GUI 安装器 / Headless systemd / Windows Service）；
     - “已执行测试”字段，要求勾选对应 WP（例如：`WP-QA-DESKTOP-01`、`WP-QA-HEADLESS-01`）。
  2. 在实际提交中，开发者在描述中引用本 WBS 的 WP 编号，说明已执行哪些 smoke / 回归项。

- **测试与验收**
  - 定期抽查 PR，确保与 WBS 中定义的路径一致；
  - 当发现 bug 源自缺失测试路径时，更新本 WBS 或 PR 模板以补齐该空白。

---

### WP-QA-REG-03：Release 资产验收与回归入口

- **概述**
  - 当 `CA-CI-ARTIFACTS` 生成新的 GitHub Release 时，你（Agent 或人类 QA）需要执行一轮最小的 Release 级别验收，确保：
    - Release 页面资产完备（包含三平台 GUI + Headless）；
    - 资产命名与分发 / Headless WBS 描述一致；
    - 至少一条桌面路径与一条 Headless 路径可以按文档完成 smoke。

- **先决条件**
  - `CA-CI-MATRIX` 最近一次运行已成功（CI 页面 `CI` workflow 全绿）；
  - `Release Artifacts` workflow (`.github/workflows/release-artifacts.yml`) 已在目标 tag 上成功完成；
  - 分发与安装器 WBS（`wbs-distribution-and-installers.md`）中关于 Release 资产来源的说明已更新。

- **实施步骤**
  1. 在 GitHub Releases 页面打开目标版本（tag 形如 `vMAJOR.MINOR.PATCH`），核对：  
     - Assets 列表中是否包含：
       - Windows / macOS / Linux 的 GUI 安装包（`.msi` / `.exe` / `.dmg` / `.app` / `.deb` / `.AppImage` 等）；
       - 各平台的 Headless 压缩包（符合 `wbs-headless-cli-package.md` 中的命名规范）。
  2. 随机选择：
     - **一条 GUI 路径**（例如 Windows NSIS 安装器或 macOS `.dmg`）；
     - **一条 Headless 路径**（例如 Linux `tar.gz` 或 Windows `.zip`）；  
     按照分发 WBS 与本文件中对应的 smoke WP（如 `WP-QA-DESKTOP-01`、`WP-QA-HEADLESS-01`）执行完整 smoke。
  3. 在验收记录中至少回填以下信息：  
     - Release tag 与 Release 页面链接；  
     - 实际下载的资产文件名列表；  
     - 对应执行的 WBS WP 编号（例如：`WP-DIST-GUI-02` + `WP-QA-DESKTOP-01`、`WP-DIST-CLI-01` + `WP-QA-HEADLESS-01`）；  
     - smoke 结论（成功 / 失败 + 失败原因简要描述）。

- **测试与验收**
  - 任意一次新的 Release 发布，都应至少完成一次该 WP 级别的验收记录；
  - 若在 Release 资产中发现：
    - 缺少某个平台的 GUI / Headless 包；
    - 命名与文档不一致；
    - smoke 无法按文档完成；  
    则需要回溯到 `CA-CI-ARTIFACTS` / 分发 WBS / CI WBS，补充或修正对应流程，并在后续迭代日志中记录修复过程。
