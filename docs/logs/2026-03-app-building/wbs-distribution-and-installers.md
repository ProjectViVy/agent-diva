---
title: Agent Diva 分发与安装器构建 WBS
---

> 使用说明（面向 Agent）：  
> 当你（Agent 或子 Agent）负责“产物分发与安装器行为”时，本文件描述你需要如何配置 Tauri 安装器、打包 Headless 压缩包，并在不同平台上完成最小验证。  
> 你可以把每个 WP 看成一个“可独立执行的任务单元”，按顺序或按需触发。

## 1. 控制账户（CA）概览

- **CA-DIST-GUI-INSTALLER：桌面 GUI 安装器产物**
  - 目标：基于 Tauri bundler，在三大平台生成可分发安装包（Windows NSIS/MSI、macOS dmg/app、Linux deb/appimage），覆盖桌面用户场景。

- **CA-DIST-CLI-PACKAGE：Headless CLI/服务分发包**
  - 目标：为服务器/无头环境提供独立的 CLI/服务二进制包（zip / tar.gz），附带配置与服务模板，便于自动化部署。

---

## 2. CA-DIST-GUI-INSTALLER：桌面 GUI 安装器产物

### 实现状态记录

- `WP-DIST-GUI-01`：进行中
  - 已落仓文件：
    - `agent-diva-gui/src-tauri/tauri.conf.json`
    - `agent-diva-gui/public/app-icon.svg`
    - `agent-diva-gui/src-tauri/icons/`
    - `scripts/ci/prepare_gui_bundle.py`
  - 当前目标：先固定多平台 Tauri 配置、图标与预处理脚本，再以 Windows 本机构建结果回填产物样例。

- `WP-DIST-GUI-02`：进行中
  - 已落仓文件：
    - `agent-diva-gui/src-tauri/windows/hooks.nsh`
    - `agent-diva-gui/src-tauri/resources/`
  - 当前策略：Windows 服务安装逻辑采用“可选开启 + 二进制存在性检查”的最小侵入方案。

- `WP-DIST-GUI-03`：进行中
  - 当前策略：优先保证 unsigned `.app` / `.dmg` 构建与目录约定，签名与 notarization 保留为后续增量工作。

- `WP-DIST-GUI-04`：进行中
  - 当前策略：优先固定 `deb` + `appimage` 目标、Ubuntu 依赖与 CI artifact 映射，再推进更细的发行版差异。

### WP-DIST-GUI-01：Tauri 安装器基本配置（多平台）

- **概述**
  - 在 `tauri.conf.*` 中统一配置安装包目标、应用标识与图标，保证不同平台产物的一致性。

- **先决条件**
  - `agent-diva-gui` 可在开发模式下正常启动；
  - Tauri 2 bundler 可在当前平台运行（按官方环境要求准备 SDK）。

- **实施步骤**
  1. 打开 `agent-diva-gui/src-tauri/tauri.conf.json`，确认以下字段已经固定，而不是继续使用初始化默认值：

     ```json
     {
       "productName": "Agent Diva",
       "identifier": "com.agentdiva.desktop",
       "bundle": {
         "active": true,
         "targets": ["nsis", "msi", "app", "dmg", "deb", "appimage"],
         "icon": [
           "icons/32x32.png",
           "icons/128x128.png",
           "icons/128x128@2x.png",
           "icons/icon.icns",
           "icons/icon.ico"
         ],
         "resources": ["resources/"]
       }
     }
     ```

  2. 在 `agent-diva-gui/public/app-icon.svg` 中维护一个**方形**源图标，然后执行：

     ```bash
     cd agent-diva-gui
     pnpm tauri icon public/app-icon.svg -o src-tauri/icons
     ```

     该命令会生成 `src-tauri/icons/32x32.png`、`icon.ico`、`icon.icns` 等 Tauri bundler 实际依赖的图标文件。

  3. 在打包 GUI 前，先构建 CLI 二进制并整理到 `src-tauri/resources/`：

     ```bash
     cargo build -p agent-diva-cli --release
     python scripts/ci/prepare_gui_bundle.py --gui-root agent-diva-gui
     ```

     当前仓库中 `agent-diva-service` 尚未落地时，脚本会继续执行，但只入包 `agent-diva(.exe)` 并在 `src-tauri/resources/manifests/gui-bundle-manifest.json` 中记录缺失状态。

  4. 在本机执行完整构建以验证配置：

     ```bash
     # Windows
     cd agent-diva-gui
     pnpm install --frozen-lockfile
     pnpm run bundle:prepare
     pnpm tauri build -- --target x86_64-pc-windows-msvc

     # macOS
     pnpm run bundle:prepare
     pnpm tauri build -- --target universal-apple-darwin

     # Linux
     pnpm run bundle:prepare
     pnpm tauri build -- --target x86_64-unknown-linux-gnu
     ```

     若本地首次执行 `pnpm install --frozen-lockfile` 失败，应先在 `agent-diva-gui` 目录执行一次：

     ```bash
     pnpm install --no-frozen-lockfile --registry https://registry.npmjs.org/
     ```

     以修正锁文件，再恢复 `--frozen-lockfile` 路径。

- **测试与验收**
  - 每个平台至少产出一个安装包文件（如 `.msi` / `.dmg` / `.deb`），且文件名中包含应用名与版本号；
  - `agent-diva-gui/src-tauri/resources/manifests/gui-bundle-manifest.json` 存在，且至少声明：
    - `agent-diva(.exe)` 已被整理到 `resources/bin/<platform>/`
    - `agent-diva-service(.exe)` 当前是否存在
  - 安装后应用出现在系统推荐的应用列表中，并能正常启动 GUI；
  - Windows 本机构建的实际 bundle 输出目录应记录为：
    - `agent-diva-gui/src-tauri/target/x86_64-pc-windows-msvc/release/bundle/`（显式 `--target`）
    - 或 `agent-diva-gui/src-tauri/target/release/bundle/`（默认目标）

---

### WP-DIST-GUI-02：Windows 安装器行为细化（含服务可选安装）

- **概述**
  - 在 Windows 上，安装器除了安装 GUI 外，还需要附带 CLI/Service 二进制，并在安装过程中提供“是否安装系统服务”的可选项。

- **先决条件**
  - 已有 `agent-diva.exe`（CLI）与 `agent-diva-service.exe`（Windows Service）构建产物；
  - 对 Tauri Windows 安装器（NSIS/MSI）的自定义 hook 机制有基本了解。

- **实施步骤**
  1. 在 `agent-diva-gui/src-tauri/tauri.conf.json` 中确保：
     - `bundle.resources` 已启用 `resources/`；
     - `bundle.windows.nsis.installerHooks` 指向 `./windows/hooks.nsh`；
     - `bundle.windows.nsis.installMode` 为 `both`，允许用户按场景选择当前用户安装或系统级安装。
  2. 通过 `scripts/ci/prepare_gui_bundle.py` 将 GUI companion binaries 统一放到：
     - `src-tauri/resources/bin/windows/agent-diva.exe`
     - `src-tauri/resources/bin/windows/agent-diva-service.exe`（可选，当前仓库尚未落地时允许缺失）
  3. 在 `agent-diva-gui/src-tauri/windows/hooks.nsh` 中使用 NSIS hook 扩展安装流程：
     - 增加一个自定义页面，提供复选框：“Install and start Agent Diva Gateway as a Windows Service”；
     - 当用户勾选时，在 `NSIS_HOOK_POSTINSTALL` 中检查以下文件是否存在：
       - `$INSTDIR\\resources\\bin\\windows\\agent-diva.exe`
       - `$INSTDIR\\resources\\bin\\windows\\agent-diva-service.exe`
     - 两个文件都存在时，执行：

       ```powershell
       agent-diva.exe service install --auto-start
       agent-diva.exe service start
       ```

     - 若服务二进制缺失，则弹出明确提示，说明本次构建仍处于“GUI 安装器已完成、服务封装待补齐”的阶段；
     - 若命令执行失败，则提示用户以管理员身份重跑安装器，或安装后手动运行 `agent-diva.exe service install --auto-start`。
  4. 目录约定固定如下：
     - GUI 主程序：`$INSTDIR\\Agent Diva.exe`（由 Tauri bundler 管理）
     - 附带 CLI/Service 二进制：`$INSTDIR\\resources\\bin\\windows\\`
     - 用户态配置目录：`%USERPROFILE%\\.agent-diva\\`
     - 服务模式数据目录：`%ProgramData%\\AgentDiva\\`

- **测试与验收**
  - 在干净 Windows VM 中测试两种路径：
    - **仅安装 GUI**：不勾选服务安装，安装完成后可打开 GUI，但系统服务列表中无 `AgentDivaGateway`；
    - **安装 GUI + 服务**：勾选复选框，安装完成后：
      - `services.msc` 中能看到并启动 `AgentDivaGateway`；
      - 重启电脑后服务仍按配置自动启动。
  - 当前仓库在 `agent-diva-service` 尚未引入前，允许出现“检测到缺少 `agent-diva-service.exe`，因此跳过服务安装”的已知提示；这属于**受控降级**，不是静默失败。

---

### WP-DIST-GUI-03：macOS dmg 与 app 签名 / 打包

- **概述**
  - 为 macOS 用户提供签名过的 `.app` 与 `.dmg`，提升安装体验与安全性（长远可接入 Notarization）。

- **先决条件**
  - 有可用的 Apple 开发者证书（长期目标），短期可先完成本地 unsigned 测试。

- **实施步骤**
  1. 确认 Tauri `bundle.targets` 中包含 `app` 与 `dmg`，并复用与 Windows 相同的 `resources/` 目录约定。
  2. 在 macOS 上运行：

     ```bash
     cargo build -p agent-diva-cli --release
     python scripts/ci/prepare_gui_bundle.py --gui-root agent-diva-gui --target-os macos
     cd agent-diva-gui
     pnpm run bundle:prepare
     pnpm tauri build -- --target universal-apple-darwin
     ```

  3. 目录与卸载策略固定如下：
     - `.app` 安装路径：`/Applications/Agent Diva.app`
     - 用户数据目录：`~/.agent-diva/`
     - 卸载动作：仅删除 `.app`，不自动删除 `~/.agent-diva/`
  4. 若已有签名证书，按 Tauri 官方文档配置签名参数；在当前阶段只保留技术预留，不把证书分发纳入仓库。
  5. 若后续接入 notarization，建议把签名与 notarization 逻辑独立到 CI release workflow，不直接塞入本地开发命令。

- **测试与验收**
  - 手动挂载 `.dmg` 并将 `.app` 拖入 `Applications`；
  - 首次启动时，系统弹窗行为符合预期（如 Gatekeeper 安全提示），并能成功进入主界面；
  - 应用卸载仅需删除 `.app`，用户数据存放在 `~/.agent-diva`，不随卸载自动移除。
  - QA 文档应明确引用本 WP 产出的 artifact 名称模式：`agent-diva-gui-macos-<arch>-<sha>`。

---

### WP-DIST-GUI-04：Linux 包管理器集成（deb / rpm / appimage）

- **概述**
  - 为 Linux 用户提供至少一种原生包格式（优先 deb + appimage），兼顾简单安装与便携运行。

- **先决条件**
  - 目标发行版使用 deb / rpm 包管理（如 Debian/Ubuntu、CentOS/RedHat）。

- **实施步骤**
  1. 确认 Tauri `bundle.targets` 中已启用 `deb` 与 `appimage`。
  2. 在 Ubuntu runner 或本地 Ubuntu LTS VM 上，先安装 Tauri 构建依赖：

     ```bash
     sudo apt-get update
     sudo apt-get install -y \
       libgtk-3-dev \
       libwebkit2gtk-4.1-dev \
       libayatana-appindicator3-dev \
       librsvg2-dev \
       patchelf
     ```

  2. 在 Ubuntu 系列上运行：

     ```bash
     cargo build -p agent-diva-cli --release
     python scripts/ci/prepare_gui_bundle.py --gui-root agent-diva-gui --target-os linux
     cd agent-diva-gui
     pnpm run bundle:prepare
     pnpm tauri build -- --target x86_64-unknown-linux-gnu
     ```

  3. 产物中应包含：
     - `.deb`：可以通过 `sudo dpkg -i xxx.deb` 安装；
     - `.AppImage`：可通过 `chmod +x xxx.AppImage && ./xxx.AppImage` 直接运行。
  4. 目录约定固定如下：
     - 应用主入口：系统桌面菜单中的 `Agent Diva`
     - 用户数据目录：`~/.agent-diva/`
     - `.deb` 卸载：`sudo apt remove agent-diva`
     - `.AppImage` 卸载：删除文件本身，保留 `~/.agent-diva/`

- **测试与验收**
  - 在目标发行版的 VM 中：
    - 验证 `dpkg -i` 安装、`apt remove` 卸载路径；
    - 验证 AppImage 可执行性；  
    - 卸载后确认用户数据目录处理符合设计。
  - QA 文档应明确引用本 WP 产出的 artifact 名称模式：`agent-diva-gui-linux-<arch>-<sha>`。

---

## 3. CA-DIST-CLI-PACKAGE：Headless CLI/服务分发包

> 技术增强版细化方案请优先参考 `docs/app-building/wbs-headless-cli-package.md`。本节保留 CA/WP 总览与最小要求，专项实施以该 companion 文档为准。

### WP-DIST-CLI-01：跨平台二进制打包格式与命名规范

- **概述**
  - 为 Headless 模式定义统一的压缩包格式与命名规范，便于在 CI/CD 与文档中引用。

- **先决条件**
  - 各平台能编译出 Release 模式的 `agent-diva` / `agent-diva-service` / 相关工具二进制。

- **实施步骤**
  1. 确定命名规范（示例）：
     - `agent-diva-{version}-{os}-{arch}.tar.gz`（Linux/macOS）；
     - `agent-diva-{version}-{os}-{arch}.zip`（Windows）。
  2. 在 CI/CD 或本地打包脚本中，使用 `tar` / `zip` 生成压缩包，并包含：
     - `bin/agent-diva` / `bin/agent-diva.exe`；
     - 如适用：`agent-diva-service.exe`；
     - 示例配置模板（`config/config.example.json`、`config/env.example`）；
     - systemd / launchd / Windows Service 模板文件（来自 Headless WBS）。

- **测试与验收**
  - 手动解压每个平台的包：
    - 在 Linux/macOS 上：`./bin/agent-diva gateway run` 可成功启动；
    - 在 Windows 上：`.\bin\agent-diva.exe gateway run` 可成功启动；
    - 示例配置与模板文件路径清晰、与文档描述一致。

---

### WP-DIST-CLI-02：随包附带的 README / Quickstart 文档

- **概述**
  - 为每个压缩包附带一份针对服务器用户的快速启动文档，覆盖安装、配置与服务化步骤。

- **先决条件**
  - 打包脚本已能将 Markdown/文本文件加入压缩包。

- **实施步骤**
  1. 编写 `README-headless.md` 模板，内容包括：
     - 解压到目标目录的命令；
     - 配置文件位置（默认 `~/.agent-diva/config.json` 或环境变量覆盖）；
     - 各平台服务化入口的链接（指向 Headless WBS 中的 systemd / launchd / Windows Service 小节）。
  2. 在打包时将 `README-headless.md` 重命名为通用 `README.md` 放入包根目录。

- **测试与验收**
  - 从压缩包中提取 `README.md`，按步骤执行一次最小安装流程，确认文档指引准确无误；
  - 更新服务模板或 CLI 参数时，同步更新该文档。

---

## 4. 与 CA-CI-ARTIFACTS / Release 资产的衔接

> 本节说明当 `CA-CI-ARTIFACTS` 工作流（`.github/workflows/release-artifacts.yml`）完成一次发布后，你（Agent）应如何从 GitHub Releases 获取 GUI 安装包与 Headless 压缩包，并将其映射回本 WBS 的 CA / WP 场景。

### 4.1 官方获取方式总览

- **发布来源**：
  - 所有桌面 GUI 安装包（Windows / macOS / Linux）与 Headless 压缩包，均来自 GitHub Releases 中由 `Release Artifacts` workflow 上传的 assets；
  - 该 workflow 复用 `CA-CI-MATRIX` 的 artifacts，整理为 `dist/gui/**` 与 `dist/headless/**` 后统一上传。
- **获取路径**：
  1. 打开对应版本的 Release 页面（tag 形如 `vMAJOR.MINOR.PATCH`）；
  2. 在 “Assets” 列表中查找：
     - GUI 安装器：由 Tauri bundler 生成的 `.msi` / `.exe` / `.dmg` / `.app` / `.deb` / `.AppImage` 等文件；
     - Headless 压缩包：遵循 `wbs-headless-cli-package.md` 中的命名规范（例如：`agent-diva-{version}-{os}-{arch}.tar.gz` 或 `.zip`）。
  3. 按本 WBS 中对应 WP 的平台说明选择资产并执行安装 / 解压。

### 4.2 GUI 安装器与 Release 资产映射

- **Windows GUI（对应 WP-DIST-GUI-01 / 02）**
  - 推荐从 Release 中选择：
    - `*.msi`：MSI 安装器路径；
    - 或 `*.exe`：NSIS 安装器路径。
  - 当你执行 `WP-DIST-GUI-02` 中的 Windows 安装器行为细化时，应明确在验收记录中注明：
    - 使用的 Release 版本（tag）；
    - 实际下载的安装器文件名。

- **macOS GUI（对应 WP-DIST-GUI-01 / 03）**
  - 推荐从 Release 中选择：
    - `*.dmg`：标准安装路径；
    - 如有必要，可直接使用 `.app` 进行开发/测试。
  - 验收时需确认：Release 中的 `.dmg` 与 QA 文档中使用的包来源一致。

- **Linux GUI（对应 WP-DIST-GUI-01 / 04）**
  - 推荐从 Release 中选择：
    - `*.deb`：原生安装包；
    - `*.AppImage`：便携运行包。
  - 验收记录中需要标记使用的是 `.deb` 还是 `.AppImage`，以便回溯到 `WP-DIST-GUI-04` 的具体路径。

### 4.3 Headless 压缩包与 Release 资产映射

- **Headless CLI / 服务包（对应 CA-DIST-CLI-PACKAGE）**
  - Release 中的 Headless 资产应遵循 `wbs-headless-cli-package.md` 所定义的命名与目录结构：
    - Linux / macOS：`agent-diva-{version}-{os}-{arch}.tar.gz`；
    - Windows：`agent-diva-{version}-{os}-{arch}.zip`。
  - 解压后应至少包含：
    - `bin/agent-diva` 或 `bin/agent-diva.exe`；
    - `README.md`（由 `WP-DIST-CLI-02` 定义）；
    - `bundle-manifest.txt` 及对应的服务模板 / 配置样例。
  - 当你执行 Headless 安装或服务化相关 WP（例如 Headless WBS 中的 systemd / Windows Service 安装脚本）时，优先从 Release 中选择对应平台的 Headless 包作为输入。
