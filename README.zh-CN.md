# Agent Diva

<img src="https://raw.githubusercontent.com/ProjectViVy/agent-diva/c565bb245cc920258d7f8c7fcd9544fbba545af7/docs/resources/diva.png" alt="Agent Diva 插画" align="right" width="300" />

[English](README.md) · [项目文档](https://projectvivy.github.io/agent-diva-docs/) · QQ 群 788599177

Agent Diva 是 Project ViVy 的 Windows x64 桌面 AI 助手。应用使用 Tauri 和 Vue，通过独立构建的 Diva Rust 核心提供流式对话、持久化本地会话，以及支持工具、技能和人格的工作区。

名字源于[《薇薇 -萤石眼之歌-》](https://en.wikipedia.org/wiki/Vivy:_Fluorite_Eye%27s_Song)。**Agent** 指尚未觉醒时的执行者，**Diva** 是舞台中央的歌姬。项目从早期 Agent Diva Rust 工作区发展而来，其代理循环受到 nanobot 启发。

<br clear="both" />

## 快速开始

1. 使用自己的 Windows 账户运行 x64 NSIS 安装程序。可以使用匹配的核心产物[构建安装包](#验证和打包)。安装按用户进行，需要 WebView2，安装程序可以下载微软的引导程序。安装包内含核心。
2. 首次启动时选择供应商，填写 API 密钥或环境变量引用并保存。文本、流式与工具三项连接测试全部通过后即可继续。
3. 描述你想要的助手，或选择内置人格。之后可在 **设置 → 核心人格** 中编辑人格文件。
4. 初始化 Python 工作台和可选的本地 MemPalace 记忆，也可到 **设置 → 运行环境** 稍后配置。选择 **开始对话** 进入聊天。

已完成初始化时，可在 **设置 → 供应商** 编辑兼容 OpenAI 或 Anthropic 的接口地址，或选择默认模型。填写接口实际提供的模型 ID。应用中录入的 API 密钥只写。内置供应商选项包括 DeepSeek、MiniMax 和阿里云百炼，百炼提供中国站标准、国际站标准和中国站 Coding Plan 三种配置。

## 功能

- 查看流式回复与工具调用结果，回答助手提问、取消当前轮次，以及重新打开已保存的会话。
- 在设置中管理本地技能、托管技能、MCP 连接及六份核心人格文件，也可打开独立的「回顾」和「任务」面板。
- 按需初始化 Python 或连接 MCP 服务器。MCP 配置可以引用宿主环境变量，Python 进程只接收运行所需的部分系统环境。
- 关闭窗口后应用隐藏到托盘，托盘菜单提供显示、设置和退出。应用在托盘运行时，已有的 Recap 与唤醒任务可以继续。退出时应用给核心十秒时间正常关闭，之后终止其管理的进程树。崩溃后可使用 **重新连接**，应用会根据已保存的提交记录核对未确认的消息。

应用以 `--approve` 启动随附的核心，自动信任固定桌面工作区内的资源。助手的其他提问仍在聊天中显示。

桌面数据保存在 `%LOCALAPPDATA%/org.projectvivy.agentdiva/` 下，分别存入 `core-home`、`workspace`、`desktop-state`、`webview2` 和 `logs`。常规升级和卸载会保留这些数据。

## 从源码开发

安装 Node.js 24 LTS（24.x 中的 24.15.0 或更新版本）、npm、稳定版 Rust、MSVC C++ 生成工具及 WebView2。详见 [Tauri 环境要求](https://v2.tauri.app/start/prerequisites/)。

```sh
git clone https://github.com/ProjectViVy/agent-diva.git
cd agent-diva
npm ci
npm run import-core -- "<path-to-extracted-core-artifact>"
npm run tauri dev
```

使用核心构建生成的匹配 Windows x64 产物，将包含 `metadata.json` 的解压目录传给导入脚本。导入会校验文件哈希与固定的 Rig 修订版本。产物包含核心可执行文件、Rust 生成的 TypeScript 类型、协议示例与版本、构建配方、Cargo.lock 和依赖许可声明。Vue 界面通过 IPC 调用 Tauri 宿主。宿主启动打包后的核心，并通过标准输入输出交换 JSON 消息。

使用 `npm run dev` 可在浏览器中预览前端。使用 `npm run dev -- --mode test --port 1422` 可打开 IPC 测试页面，页面使用模拟核心响应，无需凭据。

## 验证和打包

[CI 工作流](.github/workflows/ci.yml) 无需核心可执行文件即可运行以下检查。

```sh
npm run build
npm test
npm run test:artifact
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path bridge-tests/Cargo.toml --locked
```

如需用真实核心验证桥接，将 `DIVA_TEST_CORE` 指向匹配的核心可执行文件，再运行 `cargo test --manifest-path bridge-tests/Cargo.toml --locked -- --include-ignored`。

导入核心后，运行 `npm run tauri build -- --bundles nsis`。安装包输出到 `src-tauri/target/release/bundle/nsis/`。[打包工作流](.github/workflows/package.yml) 接受显式传入的核心 ZIP 的 HTTPS 地址与 SHA-256 校验值。

完整发布验收仍在进行。上游已在现有管理员账户下验证按用户安装、受限 PATH 启动、崩溃恢复，以及升级和卸载后的数据保留。全新标准用户环境安装、125% 和 150% 缩放下的原生 WebView2 布局、应用隐藏时的 Recap 与唤醒行为仍待发布验收。供应商兼容性取决于实际测试的接口和模型。

测试环境和接口详情见[上游验证记录](https://github.com/ProjectViVy/agent-diva/blob/8468ca94eb95d3e4e3dc493c6419a4b47431dd63/README.md#verification-limits)。

## 仓库结构

| 路径 | 内容 |
| --- | --- |
| [`src/`](src/) | Vue 桌面界面、宿主连接状态与生成的协议 |
| [`src-tauri/`](src-tauri/) | Rust 桌面宿主、核心进程管理、本地状态与 NSIS 配置 |
| [`scripts/`](scripts/) | 核心产物导入与校验 |
| [`bridge-tests/`](bridge-tests/) | Rust 桥接测试，支持使用真实核心执行集成用例 |

## 旧版工作区

旧版 Rust 多 crate 工作区、网关渠道、CLI/TUI 及其配置与构建说明，见 `dev` 分支的[旧版中文 README](https://github.com/ProjectViVy/agent-diva/blob/dev/README.zh-CN.md)与[旧版英文 README](https://github.com/ProjectViVy/agent-diva/blob/dev/README.md)。另见[旧版工作区的 MIT 许可证](https://github.com/ProjectViVy/agent-diva/blob/dev/LICENSE)。
