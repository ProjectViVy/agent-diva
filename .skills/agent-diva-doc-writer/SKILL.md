---
name: agent-diva-doc-writer
description: Focuses on reading and writing documentation for the agent-diva Rust workspace, guiding the agent to locate key project docs (AGENTS.md, CLAUDE.md, crate READMEs, MEMORY/HISTORY) and answer user questions about architecture, workflows, and usage based on those sources. Use when the user asks to explain or extend agent-diva docs, or to understand how agent-diva works.
---

# Agent Diva 文档助手

本 Skill 专门用于**阅读、理解并编写 `agent-diva` 项目的文档**，指导 Agent 如何查看仓库中的关键文件，并根据用户问题给出结构化、可靠的说明或文档输出。

## 使用场景

在下面这些场景中优先使用本 Skill：

- 用户提到 `agent-diva`、Agent Diva 架构、工作流、会话/记忆机制等，并要求说明或文档
- 用户让你“为 agent-diva 写文档 / 教程 / 指南 / FAQ / 使用说明 / 设计说明”
- 用户想了解某个 crate、模块、Provider/Channel/Tool 在 agent-diva 中是如何协作的
- 用户希望把已有零散信息（如 `AGENTS.md`、`CLAUDE.md`、README）整理成更系统的说明

## 阅读顺序与信息来源

回答与编写文档前，优先从以下位置依次获取信息：

1. **仓库根级文档**
   - `AGENTS.md`：仓库规则、项目结构、命令机制与迭代规范
   - `CLAUDE.md`：整体架构、数据流、关键 crate 职责说明
2. **crate 级别信息**
   - 各 `agent-diva-*` crate 的 `Cargo.toml`、`src/lib.rs`、模块注释、README
   - 特别关注：`agent-diva-core`、`agent-diva-agent`、`agent-diva-providers`、`agent-diva-channels`、`agent-diva-tools`、`agent-diva-cli`
3. **记忆与历史相关文件**
   - `MEMORY.md`、`HISTORY.md`（如果存在）：理解会话、记忆与长期记录的设计
4. **配置与命令**
   - 根目录或文档中提到的 `justfile`、CLI 子命令、配置文件说明（如 `~/.agent-diva/config.json`）
5. **补充源码**
   - 仅在文档不够时，再查阅具体实现代码（例如 Provider 路由、Channel 适配器、Tool 实现），用来确认行为或边界条件，但避免在文档中堆砌实现细节。

在引用这些信息时，**保持与原文档中的术语、规则一致**（例如 Provider 模型 ID 规则、Rulebook 中的约束等）。

## 文档与回答风格

面向用户输出文档或解释时，遵循以下风格：

1. **语言与语气**
   - 默认使用简体中文对用户说明
   - 风格偏向**高级工程师**：准确、克制、不过度营销
2. **结构化输出**
   - 对于“解释/说明类”问题，优先使用以下结构：

     ```markdown
     ## 概览
     [用 1–3 句话总结主题]

     ## 核心概念
     - 概念 1：简要解释
     - 概念 2：简要解释

     ## 关键模块与职责
     - 模块 A：做什么、与谁交互
     - 模块 B：做什么、与谁交互

     ## 常见使用方式
     - 场景 1：相关命令 / 配置
     - 场景 2：相关命令 / 配置

     ## 进一步阅读
     - 相关文件或文档链接列表
     ```

   - 对于“完整文档页（如指南/教程）”，可以在此基础上增加「前置条件 / 快速开始 / 注意事项 / FAQ」等小节。
3. **准确引用项目约定**
   - 说明 `just` 命令、CLI 命令、配置文件时，优先复述已有文档中的写法
   - 提及 Provider 模型 ID 时，遵守“原生 Provider 不自动加 LiteLLM 前缀”的仓库规则
   - 涉及 GUI、渠道适配器或工具时，注明是可选组件还是必需组件

## 典型工作流

### 1. 回答“agent-diva 是什么 / 怎么工作？”

步骤：

1. 读取 `AGENTS.md` 与 `CLAUDE.md`，梳理：
   - 工作区结构（各 crate 职责）
   - 数据流（Channel → Message Bus → Agent → Provider → Tool → Channel）
   - 会话与记忆机制（如 JSONL 持久化、MEMORY/HISTORY）
2. 用「概览 → 核心概念 → 关键模块 → 使用方式」结构回答用户问题
3. 如果用户关心的是某个子域（如 Provider/Channel/Tool/CLI），在对应小节展开，避免一次性解释所有细节

### 2. 为 agent-diva 写“使用指南/快速开始”

步骤：

1. 根据现有文档与 `just` 命令，梳理一个最小可运行路径：
   - 环境准备（Rust、just 等）
   - 基本构建与验证命令（如 `just build`、`just test` 或 `just ci`）
   - 最简单的运行方式（如 `just run gateway` 或 CLI 子命令示例）
2. 组织为以下结构：

   ```markdown
   # Agent Diva 快速开始

   ## 前置条件
   [环境与依赖列表]

   ## 安装与构建
   - 步骤 1
   - 步骤 2

   ## 启动一个最小示例
   - 命令
   - 预期行为/输出

   ## 常见问题
   - 问题 1：简要回答
   - 问题 2：简要回答
   ```

3. 所有命令与配置示例必须与当前仓库实际情况保持一致，如有不确定的地方，使用更中性的描述并标注假设。

### 3. 解释具体模块（如 Provider / Channel / Tool）

1. 找到对应 crate（例如 `agent-diva-providers`、`agent-diva-channels`、`agent-diva-tools`）及其 README/模块文档
2. 按以下模板总结：

   ```markdown
   ## 模块职责
   [1–2 句话说明它在整体架构中的位置]

   ## 关键类型/接口
   - 类型/trait 1：用途
   - 类型/trait 2：用途

   ## 与其他模块的关系
   - 依赖谁？被谁使用？

   ## 配置与扩展
   - 如何新增 Provider/Channel/Tool 的高层步骤
   ```

3. 避免在文档中粘贴过多实现细节，可以用「高层步骤 + 关键接口名称」方式指引用户自行查阅源码。

## 注意事项

- **不要虚构项目结构或命令**：若仓库中没有某个文件/命令，就不要当成事实写入文档
- 对于仍在演进中的部分，可以在文档中明确标注「当前设计可能变更」而不是给出过于确定的承诺
- 若用户只需要简短回答，无需自动生成过长文档；根据问题粒度调整输出长度

## 简要示例

当用户说：「帮我写一段介绍 agent-diva 架构的文档」时，可以输出类似：

```markdown
## Agent Diva 架构概览

Agent Diva 是一个基于 Rust 的多通道 AI 助手框架，通过统一的消息总线将聊天渠道（如 Telegram、Slack 等）、LLM 提供商和工具系统串联起来。

核心组件包括：
- 核心层（agent-diva-core）：提供配置加载、会话与记忆管理、事件总线等基础能力
- Agent 层（agent-diva-agent）：负责编排对话流程、构建上下文、调用技能与子 Agent
- Provider 层（agent-diva-providers）：封装各类 LLM/语音提供商，并处理模型 ID 与路由规则
- 渠道层（agent-diva-channels）：对接 Telegram、Slack 等外部渠道
- 工具层（agent-diva-tools）：实现文件系统、Shell、Web 请求等工具能力

消息从渠道进入后，会依次经过消息总线、Agent 编排、Provider 调用与工具执行，最终再通过总线返回到对应渠道。
```

生成正式文档时，可在此基础上按需扩展更多章节。

