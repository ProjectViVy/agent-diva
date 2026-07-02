---
title: Agent Diva Headless CLI Package Technical WBS
---

> 使用说明（面向 Agent）：
> 当你（Agent 或子 Agent）负责 `CA-DIST-CLI-PACKAGE` 时，优先执行本文件；它是 `docs/app-building/wbs-distribution-and-installers.md` 中 Headless 分发包部分的技术增强版落地说明。
> 本文件只覆盖 Headless CLI/服务分发包，不处理 GUI 安装器产物。

## 1. 控制账户（CA）定义

### CA-DIST-CLI-PACKAGE：Headless CLI/服务分发包

- **控制账户目标**
  - 为 Windows、macOS、Linux 生成可直接分发的 Headless 独立压缩包，满足“下载 -> 解压 -> 启动 `agent-diva gateway run` -> 可选服务化”的最小闭环。
- **控制账户负责人（CA Owner）**
  - 主责：构建与发布 Agent / 工程负责人。
  - 协作：平台服务子 Agent（Windows Service、systemd、launchd）、CI 发布 Agent、QA Agent。
- **边界**
  - 允许改动：`docs/app-building` 文档、CI workflow、打包脚本、分发目录约定、随包 README、服务模板。
  - 不允许改动：`agent-diva-core`、`agent-diva-agent`、`agent-diva-providers`、`agent-diva-channels`、`agent-diva-tools` 的核心业务行为。
  - 对 `agent-diva-cli` 仅允许做与打包路径、帮助信息、服务入口桥接有关的最小改动。
- **输入**
  - `agent-diva gateway run` 已作为标准 Headless 入口可用。
  - `CA-CI-MATRIX` 已完成，能够产出三平台基础构建工件。
  - 平台服务模板能力来自 `docs/app-building/wbs-headless-service-mode.md`。
- **输出**
  - 平台压缩包：
    - `agent-diva-{version}-windows-{arch}.zip`
    - `agent-diva-{version}-linux-{arch}.tar.gz`
    - `agent-diva-{version}-macos-{arch}.tar.gz`
  - 包内固定目录：`bin/`、`config/`、`services/`、`README.md`、`bundle-manifest.txt`
  - CI 工件发布规则与 smoke/QA 映射。
- **完成定义（CA DoD）**
  - 产物命名、目录结构、README、manifest、服务模板路径全部固定。
  - 任意目标平台的使用者按包内 `README.md` 操作，能够完成最小启动。
  - CI 能对包内容完整性、压缩格式、启动路径进行自动校验。
  - 文档与 `wbs-ci-cd-and-automation.md`、`wbs-validation-and-qa.md`、`headless-bundle-quickstart.md` 的术语和命令保持一致。

## 2. 确定性技术路线

- **构建入口**
  - 统一使用 `cargo build --release -p agent-diva-cli` 生成 Headless 主二进制。
  - Windows Service 二进制进入 Phase 2 以后再通过 `cargo build --release -p agent-diva-service` 纳入包内。
- **版本来源**
  - 统一从 workspace package metadata 读取版本号，不手写字符串。
- **打包工具**
  - Windows：PowerShell `Compress-Archive`
  - Linux / macOS：`tar -czf`
- **校验工具**
  - Windows：`Get-FileHash`
  - Linux / macOS：`sha256sum`
- **服务模板来源**
  - Windows：`service install/start/stop/uninstall` 命令约定与 `agent-diva-service.exe` 配套
  - Linux：systemd unit 与安装脚本
  - macOS：launchd plist 与安装脚本
- **阶段约束**
  - **Phase 1（当前即可交付）**：只强制要求 `agent-diva` / `agent-diva.exe`、README、manifest、配置模板。
  - **Phase 2（服务化能力接入后）**：把平台服务模板和 Windows Service 二进制并入包结构。
  - **Phase 3（发布自动化）**：Release 发布、校验 hash、回滚与升级说明固化。

```mermaid
flowchart LR
sourceBuild[ReleaseBuild] --> stageDir[StageDirectory]
stageDir --> manifestCheck[ManifestAndLayoutCheck]
manifestCheck --> archiveBuild[ArchiveBuild]
archiveBuild --> smokeRun[SmokeRun]
smokeRun --> artifactUpload[ArtifactUpload]
artifactUpload --> releasePublish[ReleasePublish]
```

## 3. 分发包契约

### 3.1 命名规范

| 平台 | 文件名格式 | 当前阶段要求 |
| --- | --- | --- |
| Windows | `agent-diva-{version}-windows-{arch}.zip` | 必需 |
| Linux | `agent-diva-{version}-linux-{arch}.tar.gz` | 必需 |
| macOS | `agent-diva-{version}-macos-{arch}.tar.gz` | 必需 |

- `arch` 当前固定为：
  - Windows: `x86_64`
  - Linux: `x86_64`
  - macOS: `universal` 或实际产物架构（若尚未做 universal，则明确写 `aarch64` / `x86_64`）
- 不允许使用 `windows-latest`、`ubuntu-latest` 这类 CI 运行器名称进入最终产物名。

### 3.2 包内目录结构

```text
agent-diva-{version}-{os}-{arch}/
  README.md
  bundle-manifest.txt
  bin/
    agent-diva
    agent-diva.exe
    agent-diva-service.exe
  config/
    config.example.json
    env.example
  services/
    README.md
    windows/
      install-service.ps1
      uninstall-service.ps1
    linux/
      agent-diva.service
      install-systemd.sh
      uninstall-systemd.sh
    macos/
      com.agentdiva.gateway.plist
      install-launchd.sh
      uninstall-launchd.sh
```

- **Phase 1 强制文件**
  - `README.md`
  - `bundle-manifest.txt`
  - `bin/agent-diva` 或 `bin/agent-diva.exe`
  - `config/config.example.json`
  - `services/README.md`
- **Phase 2 条件文件**
  - `bin/agent-diva-service.exe`
  - `services/windows/*`
  - `services/linux/*` 或 `systemd/*`（Linux 平台）
  - `services/macos/*`

- **Linux systemd 包（CA-HL-LNX-SYSTEMD 已接入）**
  - `scripts/ci/package_headless.py` 在打包 Linux 压缩包时，将 `contrib/systemd/` 下的文件打入 `systemd/` 子目录：
    - `systemd/agent-diva.service`：systemd unit 模板
    - `systemd/install.sh`：安装脚本（需 `sudo` 执行）
    - `systemd/uninstall.sh`：卸载脚本
  - `bundle-manifest.txt` 中会包含 `systemd_files=systemd/agent-diva.service,systemd/install.sh,systemd/uninstall.sh`
  - 安装流程：解压后执行 `cd <解压目录>/systemd && sudo ./install.sh`，详见 `wbs-headless-service-mode.md` 的 WP-HL-LNX-02

### 3.3 `bundle-manifest.txt` 固定字段

```text
name=agent-diva
version=0.0.0
os=windows
arch=x86_64
entrypoint=bin/agent-diva.exe gateway run
service_mode=optional
config_example=config/config.example.json
readme=README.md
```

- 该文件必须由打包脚本自动生成，不允许手工维护。
- smoke 脚本首先读取它确认入口、平台、配置模板位置。

## 4. WP-DIST-CLI-01：跨平台二进制打包格式与命名规范

- **CA / WP**
  - `CA-DIST-CLI-PACKAGE` / `WP-DIST-CLI-01`
- **WP 负责人**
  - 构建与发布 Agent
- **WP 目标**
  - 把已有 Release 二进制整理为结构一致、命名固定、可校验的跨平台 Headless 分发包。
- **输入**
  - `target/release/agent-diva` 或 `target/release/agent-diva.exe`
  - 可选：`target/release/agent-diva-service.exe`
  - 配置模板与服务模板
- **输出**
  - 三平台压缩包
  - `bundle-manifest.txt`
  - 产物 hash 文件或 hash 记录
- **完成定义（WP DoD）**
  - 目录结构与命名符合本文件 3.1 / 3.2。
  - 脚本可重复执行，不依赖人工拷贝。
  - 包内入口命令与 README 完全一致。

### Node 01.1：版本与平台参数标准化

- **责任边界**
  - 只负责生成 `{version}`、`{os}`、`{arch}` 三个构建参数，不直接参与压缩。
- **代码级实践**

```bash
cargo metadata --no-deps --format-version 1
```

```powershell
$version = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages |
  Where-Object { $_.name -eq "agent-diva-cli" } |
  Select-Object -First 1 -ExpandProperty version
```

- **测试与验收**
  - 输出版本号必须与 `Cargo.toml` 一致。
  - 最终产物名不可出现空值或 CI 平台别名。

### Node 01.2：生成 staging 目录

- **责任边界**
  - 只负责组装临时目录，不负责上传或 release。
- **代码级实践（PowerShell）**

```powershell
$root = "dist/staging/agent-diva-$version-windows-x86_64"
New-Item -ItemType Directory -Force -Path "$root/bin", "$root/config", "$root/services/windows", "$root/services" | Out-Null
Copy-Item "target/release/agent-diva.exe" "$root/bin/agent-diva.exe"
if (Test-Path "target/release/agent-diva-service.exe") {
  Copy-Item "target/release/agent-diva-service.exe" "$root/bin/agent-diva-service.exe"
}
Copy-Item "config/config.example.json" "$root/config/config.example.json" -ErrorAction SilentlyContinue
```

- **代码级实践（Bash）**

```bash
ROOT="dist/staging/agent-diva-${VERSION}-${OS}-${ARCH}"
mkdir -p "$ROOT/bin" "$ROOT/config" "$ROOT/services/linux" "$ROOT/services/macos" "$ROOT/services"
cp "target/release/agent-diva" "$ROOT/bin/agent-diva"
chmod +x "$ROOT/bin/agent-diva"
[ -f "config/config.example.json" ] && cp "config/config.example.json" "$ROOT/config/config.example.json"
```

- **测试与验收**
  - `bin/`、`config/`、`services/` 必须存在。
  - 可执行文件必须位于 `bin/`，不允许放在包根目录。

### Node 01.3：生成 `services/README.md` 与 manifest

- **责任边界**
  - 即使某个平台服务模板尚未接入，也必须在包内说明当前状态，避免用户误判“缺文件”。
- **代码级实践**

```text
# Services

- Windows: Phase 2 后提供 `install-service.ps1` 与 `agent-diva-service.exe`
- Linux: Phase 2 后提供 `agent-diva.service` 与 `install-systemd.sh`
- macOS: Phase 2 后提供 `com.agentdiva.gateway.plist` 与 `install-launchd.sh`
- 当前始终可用的入口：`bin/agent-diva gateway run`
```

```bash
cat > "$ROOT/bundle-manifest.txt" <<EOF
name=agent-diva
version=${VERSION}
os=${OS}
arch=${ARCH}
entrypoint=bin/agent-diva gateway run
service_mode=optional
config_example=config/config.example.json
readme=README.md
EOF
```

- **测试与验收**
  - `bundle-manifest.txt` 必须存在且字段完整。
  - `services/README.md` 必须明确哪些能力已接入、哪些是下一阶段接入。

### Node 01.4：压缩与校验 hash

- **责任边界**
  - 负责生成最终压缩包与 hash，不负责上传 Release。
- **代码级实践（Windows）**

```powershell
$archive = "dist/agent-diva-$version-windows-x86_64.zip"
Compress-Archive -Path "$root\*" -DestinationPath $archive -Force
Get-FileHash $archive -Algorithm SHA256 | Out-File "$archive.sha256"
```

- **代码级实践（Linux / macOS）**

```bash
ARCHIVE="dist/agent-diva-${VERSION}-${OS}-${ARCH}.tar.gz"
tar -C "dist/staging" -czf "$ARCHIVE" "agent-diva-${VERSION}-${OS}-${ARCH}"
sha256sum "$ARCHIVE" > "${ARCHIVE}.sha256"
```

- **测试与验收**
  - 压缩包可被标准工具解压。
  - `.sha256` 文件与实际文件匹配。

### Node 01.5：最小启动 smoke

- **责任边界**
  - 只验证“包可用”，不做深度业务验证。
- **代码级实践**

```powershell
Expand-Archive ".\dist\agent-diva-$version-windows-x86_64.zip" -DestinationPath ".\dist\smoke" -Force
.\dist\smoke\agent-diva-$version-windows-x86_64\bin\agent-diva.exe gateway run
```

```bash
tar -xzf "./dist/agent-diva-${VERSION}-${OS}-${ARCH}.tar.gz" -C ./dist/smoke
"./dist/smoke/agent-diva-${VERSION}-${OS}-${ARCH}/bin/agent-diva" gateway run
```

- **测试与验收**
  - 进程能够启动并输出网关启动日志。
  - 若项目已暴露健康检查端点，则进一步验证 `curl -sf http://127.0.0.1:3000/health`。

## 5. WP-DIST-CLI-02：随包附带的 README / Quickstart 文档

- **CA / WP**
  - `CA-DIST-CLI-PACKAGE` / `WP-DIST-CLI-02`
- **WP 负责人**
  - 文档 Agent + 构建 Agent
- **WP 目标**
  - 把阶段 1 的最小 Quickstart 升级为正式 Headless 分发包 README，覆盖解压、启动、配置、服务化、升级与回滚入口。
- **输入**
  - `headless-bundle-quickstart.md`
  - `wbs-headless-service-mode.md`
  - 包内目录结构与 `bundle-manifest.txt`
- **输出**
  - 随包 `README.md`
  - 与仓库文档一致的外部引用关系
- **完成定义（WP DoD）**
  - 用户只看包内 `README.md` 就能完成最小启动。
  - README 中所有路径、命令与包内结构完全一致。

### Node 02.1：统一 README 章节

- **固定章节**
  - Bundle Contents
  - Minimum Run Path
  - Configuration
  - Optional Service Mode
  - Upgrade / Rollback
  - Verification
  - Pointers To Full WBS
- **测试与验收**
  - README 不允许只保留占位文案，必须是“可执行说明”。

### Node 02.2：给出平台最短启动路径

- **代码级实践**

```powershell
cd C:\AgentDiva
.\bin\agent-diva.exe gateway run
```

```bash
cd /opt/agent-diva
chmod +x ./bin/agent-diva
./bin/agent-diva gateway run
```

- **测试与验收**
  - Windows 与 Linux/macOS 命令都基于 `bin/` 子目录，不允许与实际包布局脱节。

### Node 02.3：配置文件指引

- **固定约定**
  - 包内只提供 `config/config.example.json` 和 `config/env.example` 模板。
  - 实际运行配置仍以用户目录 `~/.agent-diva/config.json` 或环境变量覆盖为准。
- **代码级实践**

```text
1. 复制 `config/config.example.json` 到用户配置目录。
2. 根据 provider/channel/manager 配置修改。
3. 使用环境变量 `AGENT_DIVA__*` 覆盖敏感项。
```

- **测试与验收**
  - README 必须说明“模板文件不自动生效”，避免用户误以为解压即完成配置。

### Node 02.4：服务化入口与状态说明

- **固定约定**
  - README 只提供“如何进入服务化”的入口，不在包内复制完整服务设计文档。
  - Windows 指向 `service install/start/stop/uninstall` 或后续 `services/windows/*.ps1`
  - Linux 指向 systemd unit 安装路径
  - macOS 指向 launchd plist 安装路径
- **测试与验收**
  - 当某平台服务模板尚未随包发布时，README 必须明确标注 `Phase 2`，不允许假设已可用。

### Node 02.5：升级、回滚、卸载

- **固定约定**
  - 升级：解压新版本到新目录，切换服务或启动脚本指向。
  - 回滚：保留旧版本目录，通过恢复服务目标路径完成。
  - 卸载：删除程序目录，不自动删除 `~/.agent-diva` 用户数据目录。
- **测试与验收**
  - README 需明确“应用目录”和“用户数据目录”是两套概念。

## 6. CI 工件与 Release 衔接

- **目标**
  - 让 `CA-DIST-CLI-PACKAGE` 直接挂接到 `CA-CI-ARTIFACTS`，构成“构建 -> 打包 -> 上传 -> 发布”闭环。

### 6.1 CI Job 固定步骤

```yaml
headless-package:
  needs: rust-check
  runs-on: ${{ matrix.os }}
  strategy:
    matrix:
      include:
        - os: windows-latest
          artifact_os: windows
          artifact_arch: x86_64
        - os: ubuntu-latest
          artifact_os: linux
          artifact_arch: x86_64
        - os: macos-latest
          artifact_os: macos
          artifact_arch: universal

  steps:
    - uses: actions/checkout@v4
    - uses: actions-rust-lang/setup-rust-toolchain@v1
      with:
        toolchain: stable
    - name: Build CLI
      run: cargo build --release -p agent-diva-cli
    - name: Package headless bundle
      run: ./scripts/dist/package-headless.${{ runner.os == 'Windows' && 'ps1' || 'sh' }}
    - name: Upload artifact
      uses: actions/upload-artifact@v4
      with:
        name: headless-${{ matrix.artifact_os }}-${{ matrix.artifact_arch }}
        path: dist/*
```

### 6.2 Release 发布规则

- 只在 tag `v*.*.*` 触发。
- 发布资产必须同时包含：
  - 主压缩包
  - `.sha256`
- 如果缺任一文件，release job 直接失败。

### 6.3 失败门禁

- 任一以下条件触发，job 必须失败：
  - 产物名不符合命名规范
  - 包内缺少 `README.md`
  - 包内缺少 `bundle-manifest.txt`
  - 入口二进制不在 `bin/`
  - smoke 启动失败
  - hash 文件缺失或校验不一致

## 7. QA / 自动化验证映射

| 本文节点 | 对应 QA / CI 节点 | 验证目标 |
| --- | --- | --- |
| `Node 01.5` | `CA-CI-SMOKE` / `WP-CI-SMOKE-02` | Headless 包能启动 |
| `WP-DIST-CLI-01` | `CA-QA-SMOKE-HEADLESS` | 解压、启动、服务模板路径正确 |
| `WP-DIST-CLI-02` | `WP-QA-REG-02` | README/回归范围标注正确 |
| CA 级 DoD | `WP-QA-REG-01` | `just fmt-check`、`just check`、`just test` 执行记录完整 |

## 8. 阶段里程碑

### 里程碑 M1：Phase 1 最小可分发包

- **范围**
  - `bin/agent-diva(.exe)`
  - `README.md`
  - `bundle-manifest.txt`
  - `config/config.example.json`
  - `services/README.md`
- **验收标准**
  - 三平台压缩包可生成
  - 最小启动 smoke 通过

### 里程碑 M2：Phase 2 服务模板入包

- **范围**
  - Windows `agent-diva-service.exe` 与 `services/windows/*`
  - Linux `services/linux/*`
  - macOS `services/macos/*`
- **验收标准**
  - 包内服务模板与 `wbs-headless-service-mode.md` 一致
  - 至少一个平台完成服务安装 smoke

### 里程碑 M3：Phase 3 Release 固化

- **范围**
  - Release 发布
  - hash 公布
  - 升级/回滚说明进入随包 README
- **验收标准**
  - Tag 发布后自动上传全部资产
  - 运维/QA 可直接消费 Release 页面工件
