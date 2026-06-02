# Nanobot 对标差异与多模态调研

本文记录 `.workspace/nanobot` 与当前 `agent-diva` 的能力差异，重点回答两个问题：

1. nanobot 目前有哪些能力已经具备，而 `agent-diva` 尚未具备或尚未闭环。
2. 多模态能力上，`agent-diva` 还缺哪一层统一抽象与工程闭环。

本文只基于当前仓库和 `.workspace/nanobot` 的只读调研，不依赖外部网络信息。

## 结论摘要

- `agent-diva` 已覆盖 nanobot 的一批基础能力，包括 `MCP`、`Cron`、`Heartbeat`、`Subagent`、技能系统、基础多通道和 Web 工具。
- 真正值得优先补齐的差异，不在“有没有 Agent Loop”，而在“产品闭环与扩展机制”：
  - Provider OAuth 登录闭环
  - 通用 channel login 框架
  - 外部 channel plugin 机制
  - `WeCom` / `Mochat` 通道
  - 更完整的多模态统一抽象
- `docs/logs` 当前没有直接记录 `nanobot` 或 `.workspace/nanobot` 的专项迭代；nanobot 相关定位主要出现在 README 和迁移文档中。

## 日志排查结论

对 `docs/logs` 的搜索结果显示：

- 没有直接命中 `nanobot` 或 `.workspace/nanobot` 的专项开发日志。
- 现有日志里明确出现的参考工程主要是 `openclaw`、`zeroclaw`、`Shannon`。
- 仓库里关于 nanobot 的明确定位主要来自：
  - `README.md`
  - `README.zh-CN.md`
  - `docs/dev/migration.md`

因此，本次对标结论应视为“仓库现状调研文档”，而不是“已有日志中的 nanobot 专项记录整理”。

## nanobot 已有、agent-diva 当前没有或没闭环的能力

### 1. 外部 Channel Plugin 机制

nanobot 已支持通过 Python entry points 动态发现并加载外部 channel plugin，且有完整插件开发文档。

`agent-diva` 当前迁移文档明确写明 Rust 版仍以静态编译为主，插件机制属于未来规划。

判断：

- nanobot：已实现
- agent-diva：未实现

工程意义：

- 这决定了 channel 生态扩展速度。
- 对 `agent-diva` 的“Pro 化 + 易扩展”定位影响很大。

证据：

- `.workspace/nanobot/docs/CHANNEL_PLUGIN_GUIDE.md`
- `.workspace/nanobot/nanobot/channels/registry.py`
- `docs/dev/migration.md`

### 2. 通用 Provider OAuth 登录闭环

nanobot 已将 `openai-codex` 作为真实 provider 能力接入，并提供登录流。

`agent-diva` 虽然文档里已经写了 `agent-diva provider login <provider>`，但实现仍是 placeholder。

判断：

- nanobot：已实现
- agent-diva：文档先行，能力未闭环

工程意义：

- 这是最明显的“用户看起来有命令，实际不能用”的缺口。
- 应优先修复文档与产品行为不一致的问题。

证据：

- `.workspace/nanobot/nanobot/providers/openai_codex_provider.py`
- `.workspace/nanobot/README.md`
- `agent-diva-cli/src/provider_commands.rs`
- `.workspace/agent-diva-docs/content/docs/cli/index.md`

### 3. 通用 Channel Login 框架

nanobot 的 channel 基类定义了 `login(force=False)` 这一能力，插件和内建 channel 都能复用该机制。

`agent-diva` 当前只有 WhatsApp 做了登录流，其他 channel 会直接提示“not implemented yet”。

判断：

- nanobot：已实现
- agent-diva：仅部分实现

工程意义：

- 这会直接影响二维码类通道和后续扩展通道的接入成本。
- 建议抽象到 `agent-diva-channels` 的统一 trait 层，而不是继续在 CLI 里按通道分支堆逻辑。

证据：

- `.workspace/nanobot/nanobot/channels/base.py`
- `.workspace/nanobot/docs/CHANNEL_PLUGIN_GUIDE.md`
- `agent-diva-cli/src/main.rs`

### 4. WeCom 与 Mochat 通道

nanobot 明确支持 `WeCom` 和 `Mochat`，并带有独立实现。

`agent-diva` 当前公开文档中的通道列表未包含这两个通道。

判断：

- nanobot：已实现
- agent-diva：未实现

工程意义：

- 面向企业微信和私域自动化场景时，这两个通道有实际价值。

证据：

- `.workspace/nanobot/nanobot/channels/wecom.py`
- `.workspace/nanobot/nanobot/channels/mochat.py`
- `.workspace/nanobot/README.md`
- `.workspace/agent-diva-docs/content/docs/channels/index.md`

### 5. 更完整的 Provider 覆盖面

nanobot 当前更明确地支持：

- `Azure OpenAI`
- `VolcEngine`
- `OpenAI Codex`

`agent-diva` 当前公开文档和实现闭环上仍不完整，其中 `azure` 在 provider 清单中仍为注释态，`openai-codex` 仅文档入口存在。

判断：

- nanobot：已实现
- agent-diva：部分未实现、部分未闭环

工程意义：

- 这直接影响企业部署场景和中国区开发者使用体验。

证据：

- `.workspace/nanobot/nanobot/providers/registry.py`
- `.workspace/nanobot/README.md`
- `agent-diva-providers/src/providers.yaml`
- `.workspace/agent-diva-docs/content/docs/providers/index.md`

### 6. 公共技能注册表入口（ClawHub）

nanobot 已提供 `ClawHub` skill，用于搜索和安装公共技能。

`agent-diva` 当前已有技能系统，但没有对应的公共技能注册表接入证据。

判断：

- nanobot：已实现
- agent-diva：未实现

工程意义：

- 这会显著提升“装完即用”的体验。
- 也有利于围绕技能体系形成分发能力。

证据：

- `.workspace/nanobot/nanobot/skills/clawhub/SKILL.md`
- `.workspace/nanobot/README.md`

## 不应算作 nanobot 独有的能力

以下能力 `agent-diva` 已经具备，不应误判为 nanobot 独有：

- `MCP`
- `Cron`
- `Heartbeat`
- `Subagent`
- 技能系统
- `Matrix`
- `WhatsApp`
- `thinking_blocks`
- Web 搜索和抓取

因此，对标重点不应再放在“补一个基础 Agent 框架”，而应放在“工程闭环、生态扩展、统一抽象”。

## 优先级建议

### P0

#### P0-1. 补齐 Provider OAuth 登录闭环

优先对象：

- `openai-codex`
- 后续可能扩展到其他 OAuth/device-flow provider

建议改动：

- `agent-diva-cli`
- `agent-diva-providers`
- `agent-diva-core` 中配置落盘与令牌持久化部分

验收标准：

- `agent-diva provider login openai-codex` 可以真实完成登录
- 登录后 `provider status` / `provider models` 行为可用
- 文档与实现一致

#### P0-2. 提炼通用 `channels login` 机制

建议改动：

- 在 `agent-diva-channels` trait 层定义交互登录能力
- CLI 只做统一路由，不再为每个 channel 写独立硬编码分支

验收标准：

- `agent-diva channels login <channel>` 具备统一入口
- 至少 `whatsapp` 迁移到统一机制
- 新通道接入交互登录不需要修改 CLI 主流程

#### P0-3. 设计 Rust 版外部 Channel Plugin 机制

建议先做设计文档与最小原型，不必一开始就追求完整动态链接。

建议方向：

- Phase 1：注册表 + 外部进程桥接
- Phase 2：WASM/ABI 稳定化

验收标准：

- 明确插件生命周期、配置注入、消息总线边界、错误隔离方式
- 形成可实施的最小方案文档

### P1

#### P1-1. 接入 `WeCom` / `Mochat`

建议改动：

- `agent-diva-channels`
- `agent-diva-core` 配置 schema
- 对应文档

#### P1-2. 补 Provider 覆盖面

建议优先顺序：

1. `Azure OpenAI`
2. `VolcEngine`
3. `OpenAI Codex` 完整 provider 闭环

#### P1-3. 评估公共技能注册表

建议先做只读安装型注册表，不要一开始引入复杂的线上执行逻辑。

### P2

#### P2-1. 升级 onboarding 体验

目标：

- 更强的模型补全
- 更明确的 provider 差异提示
- 更接近 nanobot wizard 的引导体验

#### P2-2. 渠道富交互细节打磨

例如：

- reply context
- Slack reaction
- Feishu 富文本和 code block 表现
- Telegram 媒体细节

这些有价值，但不应排在 P0/P1 之前。

## 多模态能力调研

## 现状判断

nanobot 的多模态更接近“统一能力层”设计；`agent-diva` 则更像“各个 channel 分别支持一部分媒体能力”，整体还没有完全打通到统一的 Agent 输入输出抽象。

### nanobot 的多模态特征

#### 1. 图片可进入模型上下文

nanobot 会把入站图片文件读取为 base64 `image_url` 内容块，再与文本一起组成用户消息。

这意味着支持视觉的模型可以直接看到用户上传图片，而不是只看到占位文本。

证据：

- `.workspace/nanobot/nanobot/agent/context.py`

#### 2. 文件系统工具可直读图片

`read_file` 在读到图片时，不会简单报“二进制文件不可读”，而是返回图片内容块。

证据：

- `.workspace/nanobot/nanobot/agent/tools/filesystem.py`

#### 3. `web_fetch` 可直接处理图片 URL

对图片 URL，nanobot 会直接抓取图片并返回图片内容块，而不是只做文本摘要。

证据：

- `.workspace/nanobot/nanobot/agent/tools/web.py`

#### 4. `message` 工具将附件视为一等输出

nanobot 明确把图片、文档、音频、视频作为统一附件输出能力，而不是 channel 私有能力。

证据：

- `.workspace/nanobot/nanobot/agent/tools/message.py`

#### 5. 基类支持语音转写入口

channel 基类直接提供音频转写方法，频道只需下载音频文件即可复用。

证据：

- `.workspace/nanobot/nanobot/channels/base.py`

### agent-diva 的多模态现状

#### 1. 已有附件输出能力

`message` 工具已经支持 `media` 参数，可附带文件路径发送。

证据：

- `agent-diva-tools/src/message.rs`

#### 2. 部分 channel 已有媒体处理

当前仓库中可见的多媒体能力包括：

- WhatsApp 语音转写
- Matrix 媒体上传
- DingTalk 图片/视频/文件发送
- Email 附件发送

证据：

- `agent-diva-channels/src/whatsapp.rs`
- `agent-diva-channels/src/matrix.rs`
- `agent-diva-channels/src/dingtalk.rs`
- `agent-diva-channels/src/email.rs`

#### 3. Agent 输入上下文仍以纯文本为主

`agent-diva-agent/src/context.rs` 当前没有像 nanobot 一样，把入站图片统一编码成多模态消息块后注入模型。

这意味着：

- Channel 收到了图片，不等于模型真正看到了图片
- 多模态能力目前更多停留在 channel 和附件层，而非统一推理层

证据：

- `agent-diva-agent/src/context.rs`

#### 4. 文件工具仍偏文本导向

`read_file` 目前读取文本文件为主，不支持图片直读返回图片块。

证据：

- `agent-diva-tools/src/filesystem.rs`

## 多模态差距总结

当前主要缺口不在“能不能收附件”，而在下面四层：

### 1. 统一媒体抽象缺失

当前 `InboundMessage.media` / `OutboundMessage.media` 更像是“字符串路径列表”，而不是带类型、MIME、来源、转写状态的统一媒体对象。

### 2. 模型输入层未统一支持图片

即使 channel 已经下载了图片，Agent 也没有统一把图片转成 provider 可消费的视觉输入格式。

### 3. 工具层未把图片作为一等内容块

`read_file` / `web_fetch` 仍以文本抽取为主，图片并未进入同一工具语义层。

### 4. 通道能力与 Agent 能力尚未解耦

当前许多多模态能力存在于 channel 内部逻辑中，未上升为跨通道复用的统一能力。

## 多模态开发建议

### MM-P0. 统一媒体模型

建议新增统一媒体结构，至少包含：

- `kind`: `image | audio | video | file`
- `path`
- `mime`
- `source_url`
- `transcription`
- `metadata`

建议影响范围：

- `agent-diva-core`
- `agent-diva-channels`
- `agent-diva-tools`
- `agent-diva-agent`

### MM-P1. 图片进入 Agent 上下文

第一阶段优先只做图片：

- 入站图片下载到本地
- 在上下文构建时转成 provider 可识别的图片消息块
- 不支持视觉的 provider 自动降级为文本提示

建议影响范围：

- `agent-diva-agent`
- `agent-diva-providers`

### MM-P2. 工具层图片一等支持

建议补齐：

- `read_file` 读图
- `web_fetch` 取图
- `message` 统一附件发送语义

这样可以使 Agent 在“看图、抓图、发图”三条链路上语义一致。

### MM-P3. 语音与视频逐步提升

第二阶段再考虑：

- 通用音频转写抽象
- 语音消息统一转写回填
- 视频先做附件转发，不急于直接视频理解

## 建议的实施顺序

推荐顺序如下：

1. `provider login` 闭环
2. `channels login` 抽象
3. 多模态统一媒体模型
4. 图片进入 Agent 上下文
5. 外部 channel plugin 设计与原型
6. `WeCom` / `Mochat`
7. Provider 扩展
8. 公共技能注册表

## 参考证据索引

### agent-diva

- `README.md`
- `README.zh-CN.md`
- `docs/dev/migration.md`
- `agent-diva-cli/src/provider_commands.rs`
- `agent-diva-cli/src/main.rs`
- `agent-diva-agent/src/context.rs`
- `agent-diva-tools/src/message.rs`
- `agent-diva-tools/src/filesystem.rs`
- `agent-diva-channels/src/whatsapp.rs`
- `agent-diva-channels/src/matrix.rs`
- `agent-diva-channels/src/dingtalk.rs`
- `agent-diva-channels/src/email.rs`
- `.workspace/agent-diva-docs/content/docs/channels/index.md`
- `.workspace/agent-diva-docs/content/docs/providers/index.md`
- `.workspace/agent-diva-docs/content/docs/cli/index.md`

### nanobot

- `.workspace/nanobot/README.md`
- `.workspace/nanobot/docs/CHANNEL_PLUGIN_GUIDE.md`
- `.workspace/nanobot/nanobot/channels/base.py`
- `.workspace/nanobot/nanobot/channels/registry.py`
- `.workspace/nanobot/nanobot/channels/wecom.py`
- `.workspace/nanobot/nanobot/channels/mochat.py`
- `.workspace/nanobot/nanobot/providers/registry.py`
- `.workspace/nanobot/nanobot/providers/openai_codex_provider.py`
- `.workspace/nanobot/nanobot/agent/context.py`
- `.workspace/nanobot/nanobot/agent/tools/filesystem.py`
- `.workspace/nanobot/nanobot/agent/tools/web.py`
- `.workspace/nanobot/nanobot/agent/tools/message.py`
- `.workspace/nanobot/nanobot/skills/clawhub/SKILL.md`
