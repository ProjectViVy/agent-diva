# Agent Diva (Rust)

一个轻量、可扩展的个人 AI 助手框架，使用 Rust 构建。
本仓库包含多 crate 工作区，覆盖核心能力、提供商集成、渠道适配、内置工具与 CLI。

[![CI](https://github.com/HKUDS/agent-diva/actions/workflows/ci.yml/badge.svg)](https://github.com/HKUDS/agent-diva/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## 为什么是 Agent Diva

- 启动快、资源占用低
- 模块化架构（渠道 / 提供商 / 工具可替换）
- 一流 CLI 体验，适合本地工作流与自动化
- 持久化会话与记忆管理
- 通过 Markdown 加载技能，扩展能力简单

## 工作区结构

```
agent-diva/
|-- agent-diva-core/       # 共享配置、记忆/会话、定时任务、心跳、事件总线
|-- agent-diva-agent/      # 代理循环、上下文组装、技能/子代理流程
|-- agent-diva-providers/  # 大模型/转写提供商抽象与实现
|-- agent-diva-channels/   # 渠道适配（Slack/Discord/Telegram/Email/QQ 等）
|-- agent-diva-tools/      # 内置工具（文件/命令行/网页/定时/进程）
|-- agent-diva-cli/        # CLI 入口
|-- agent-diva-migration/  # 旧版本迁移工具
`-- agent-diva-gui/        # 可选 GUI（视构建配置）
```

## 依赖

- Rust 1.70+（通过 rustup 安装）
- 可选：`just`（工作区命令入口）

## 快速开始

克隆并构建：

```bash
git clone https://github.com/HKUDS/agent-diva.git
cd agent-diva
cargo build --all
```

安装 CLI：

```bash
cargo install --path agent-diva-cli
```

初始化配置：

```bash
agent-diva onboard
```

## 配置

默认配置文件：

```
~/.agent-diva/config.json
```

支持环境变量覆盖（结构化与别名同时可用），例如：

```
AGENT_DIVA__AGENTS__DEFAULTS__MODEL=...
OPENAI_API_KEY=...
ANTHROPIC_API_KEY=...
```

## 使用

```bash
# 启动网关（代理 + 已启用的渠道）
agent-diva gateway

# 发送单条消息
agent-diva agent --message "Hello, Agent Diva!"

# 启动交互式 TUI
agent-diva tui

# 查看状态
agent-diva status
```

## 开发

常用命令（优先使用 `just`）：

```bash
# 查看可用命令
just

# 一键格式化 + lint + 测试
just ci

# 运行全部测试
just test
```

不使用 `just` 时：

```bash
cargo fmt --all
cargo clippy --all -- -D warnings
cargo test --all
```

## 文档

- 架构：`docs/architecture.md`
- 开发：`docs/development.md`
- 迁移：`docs/migration.md`

## 贡献

贡献指南见 `CONTRIBUTING.md`。提交前请运行 `just ci`，并保持 PR 聚焦单一主题。

## 许可证

MIT，详见 `LICENSE`。

## 致谢

本 Rust 工作区是对原 Agent Diva 项目的重写实现。
