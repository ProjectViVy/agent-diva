# 多模态开发项目管理计划

> 日期：2026-06-01
> 阶段：开发项目管理计划
> 范围：先交付图片识别 / vision 输入的最小完整垂直切片，再评估音频、视频、图片生成等后续多模态能力。

## 1. 项目目标

本项目的第一目标不是一次性建设“完整多模态平台”，而是先完成一个可验证、可上线、可回滚的图片识别能力：

```text
用户在 GUI 中附加图片并输入文本问题后，agent-diva 能将文本和图片作为同一轮用户消息发送给支持 vision 的模型；不支持 vision 的模型必须给出明确降级提示，而不是让图片退化成文本占位符。
```

该目标基于现有调研：

- `docs/dev/multimodal/image-recognition-prephase-analysis-plan.md`
- `docs/dev/multimodal/cross-project-comparison-and-diva-path.md`
- `docs/dev/multimodal/claude-code-vision-analysis.md`
- `docs/dev/multimodal/hermes-agent-vision-analysis.md`
- `docs/dev/multimodal/genericagent-opanfang-vision-analysis.md`

## 2. 项目边界

### 2.1 本阶段范围

- GUI 图片附件发送。
- 图片附件元数据持久化。
- provider-neutral 的消息内容块类型。
- 当前模型 vision 能力判断。
- OpenAI-compatible vision 请求序列化。
- text-only 模型的明确降级提示。
- 最小 GUI 显示与 smoke test。

### 2.2 本阶段不做

- 音频转写。
- 视频理解。
- 图片生成。
- TTS / 实时语音。
- 全量媒体库或附件管理器。
- 全 channel 的媒体重构。
- OCR-only 作为主路径。
- 大规模 session durability 重写。

这些能力进入后续 roadmap，不阻塞第一阶段图片识别交付。

## 3. 当前状态摘要

### 3.1 已具备基础

| 能力 | 当前状态 | 主要位置 |
| --- | --- | --- |
| 文件上传 | GUI 已能上传文件并拿到 file_id | `agent-diva-gui/src/components/ChatView.vue` |
| 文件服务 | manager 可存储上传文件 | `agent-diva-manager/src/file_service.rs` |
| 附件模型 | `FileAttachment` 已有 MIME、大小、file_id | `agent-diva-core/src/attachment.rs` |
| 消息总线 | `InboundMessage.media` 可携带附件 ID | `agent-diva-core/src/bus/events.rs` |
| agent loop 附件加载 | 已能读取小文本附件 | `agent-diva-agent/src/agent_loop/loop_turn.rs` |

### 3.2 核心缺口

| 缺口 | 当前问题 | 项目内解决方式 |
| --- | --- | --- |
| Provider 消息内容 | `Message.content` 仍是 `String` | 引入 `MessageContent` / `MessageContentPart` |
| 图片进入模型上下文 | 非文本附件只变成文本占位 | 图片附件转为结构化 image part |
| 模型能力判断 | 无 `vision` capability | 引入 `ModelCapabilities` |
| 会话历史附件 | `ChatMessage` 只保存文本 content | 保存 `FileAttachmentRef` 元数据 |
| provider 序列化 | 只支持纯文本 content | OpenAI-compatible adapter 支持 content array |
| GUI 能力感知 | 不知道当前模型是否可看图 | 首阶段至少警告，后续可禁用发送 |

## 4. 交付策略

采用“最小完整垂直切片”策略：

```text
GUI uploadFile
-> manager file service
-> InboundMessage.media
-> agent loop typed content assembly
-> provider capability guard
-> OpenAI-compatible vision serialization
-> session attachment metadata
-> GUI smoke validation
```

原则：

- 每个阶段都必须留下可测试的中间状态。
- 不只改类型，必须保证链路贯通。
- 保持 text-only 路径兼容。
- 不改变 native provider raw model ID；只有真实 LiteLLM-style gateway 才允许 provider/model 前缀重写。
- 每个阶段都更新 `docs/logs`。

## 5. WBS 与活动拆解

### M0. 项目启动与计划冻结

目标：把调研内容转换为可执行项目计划。

| 活动 ID | 活动 | 产出 | 验收标准 |
| --- | --- | --- | --- |
| M0-A1 | 整理现有调研材料 | 文档索引与范围确认 | 明确第一阶段只做图片识别 |
| M0-A2 | 定义项目边界 | 范围 / 非范围清单 | 音频、视频、图片生成不进入首阶段 |
| M0-A3 | 制定 WBS | 本项目管理计划 | 活动可分配、可验收 |
| M0-A4 | 建立风险台账 | 风险与缓解表 | 每个高风险项有控制措施 |
| M0-A5 | 建立迭代日志 | `docs/logs/.../v0.0.2-project-management-plan` | summary / verification / release / acceptance 完整 |

### M1. 类型契约与兼容层

目标：让 provider 层能表达文本 + 图片内容，同时不破坏现有文本调用。

| 活动 ID | 活动 | 产出 | 验收标准 |
| --- | --- | --- | --- |
| M1-A1 | 设计 `MessageContent` | `Text` / `Parts` enum | `Message::user("text")` 仍可用 |
| M1-A2 | 设计 `MessageContentPart` | `Text` / `ImageUrl` / `ImageFile` / `ImageData` | 能表达本地 file_id 和 data URI |
| M1-A3 | 增加序列化兼容测试 | provider base 单元测试 | 旧 JSON 纯字符串可读，新结构可写 |
| M1-A4 | 更新 provider 内部引用 | 编译通过 | text-only provider 行为不变 |
| M1-A5 | 记录类型契约决策 | 迭代日志 | 明确类型放置位置与兼容策略 |

建议负责人：后端 / provider。

依赖：M0 完成。

### M2. 会话附件元数据

目标：历史消息能知道用户当时附加了哪些图片，但不把图片 bytes 写入 JSONL。

| 活动 ID | 活动 | 产出 | 验收标准 |
| --- | --- | --- | --- |
| M2-A1 | 定义 `FileAttachmentRef` | file_id、filename、mime_type、size | 字段足够恢复 GUI 标记 |
| M2-A2 | 扩展 `ChatMessage` | `attachments: Option<Vec<FileAttachmentRef>>` | 老消息无 attachments 时可读 |
| M2-A3 | 保存用户消息附件 | session store 写入附件元数据 | 发送图片后历史记录保留 file_id |
| M2-A4 | 历史读取兼容 | `get_history()` 不破坏现有上下文 | text-only 历史行为不变 |
| M2-A5 | 增加 session 测试 | 单元 / 集成测试 | 老格式兼容、新格式可 round-trip |

建议负责人：后端 / session。

依赖：M1 的类型方向确认。

### M3. Agent Loop 图片组装

目标：把 `InboundMessage.media` 中的图片附件转成同一轮用户消息里的结构化 image part。

| 活动 ID | 活动 | 产出 | 验收标准 |
| --- | --- | --- | --- |
| M3-A1 | 区分文本与图片附件 | MIME 判断逻辑 | 文本附件保留现有 inline 行为 |
| M3-A2 | 图片附件转 image part | `MessageContentPart::ImageFile` 或 `ImageData` | 文本 prompt 和图片在同一 user message |
| M3-A3 | 附件读取错误处理 | 清晰错误或降级文本 | file_id 不存在时不 panic |
| M3-A4 | token 估算策略 | 首阶段固定图片 token 成本或保守占位 | 不影响上下文截断稳定性 |
| M3-A5 | agent loop 单测 | prompt + PNG 附件组装测试 | 输出包含 text part 和 image part |

建议负责人：agent loop。

依赖：M1、M2。

### M4. 模型能力与降级策略

目标：避免向 text-only 模型发送无效 vision payload。

| 活动 ID | 活动 | 产出 | 验收标准 |
| --- | --- | --- | --- |
| M4-A1 | 定义 `ModelCapabilities` | `vision` / `tools` / `reasoning` | 默认 `vision = false` |
| M4-A2 | 建立静态 capability 判断 | 常见 vision model 白名单或配置项 | 未知模型保守判定为不支持 |
| M4-A3 | provider 调用前 guard | 阻止无效 image payload | text-only 模型给出明确提示 |
| M4-A4 | GUI/API 暴露能力 | 至少 backend 可返回当前模型能力 | GUI 可用于显示警告 |
| M4-A5 | 降级测试 | 不支持 vision 时不发送 image block | 用户能看到清晰原因 |

建议负责人：provider / manager。

依赖：M1、M3。

### M5. OpenAI-Compatible Vision 序列化

目标：首个真实 provider 路径可以发送 vision content array。

| 活动 ID | 活动 | 产出 | 验收标准 |
| --- | --- | --- | --- |
| M5-A1 | 定义 OpenAI-compatible content array 映射 | text / image_url JSON | 请求中 content 是数组 |
| M5-A2 | `ImageFile` 转 data URI | 读取 bytes + MIME + base64 | PNG/JPEG 可发送 |
| M5-A3 | 限制图片大小 | max bytes / MIME 白名单 | 超限时明确报错 |
| M5-A4 | 保持 model ID 安全 | 请求模型字段测试 | native endpoint 不加 LiteLLM 前缀 |
| M5-A5 | provider request 测试 | outbound JSON shape 测试 | 同一 user message 包含 text 和 image_url |

建议负责人：provider。

依赖：M1、M3、M4。

### M6. GUI 最小体验

目标：用户能看见自己附加了图片，并知道当前模型能否处理图片。

| 活动 ID | 活动 | 产出 | 验收标准 |
| --- | --- | --- | --- |
| M6-A1 | 图片附件 chip | 文件名、类型、大小显示 | 图片和普通文件可区分 |
| M6-A2 | 发送时携带 file_id | 已有 attachments 流程确认 | backend 收到 `attachments` |
| M6-A3 | pending message 附件显示 | 乐观消息展示附件 | 发送后不丢附件视觉标记 |
| M6-A4 | 历史消息附件标记 | reload 后显示“包含图片” | 不需要完整媒体 viewer |
| M6-A5 | text-only 模型警告 | 发送前或发送后明确提示 | 用户知道需切换 vision 模型 |

建议负责人：GUI / manager。

依赖：M2、M4。

### M7. 验证、验收与发布准备

目标：证明最小切片可用，并把验证路径写入迭代日志。

| 活动 ID | 活动 | 产出 | 验收标准 |
| --- | --- | --- | --- |
| M7-A1 | 格式检查 | `just fmt-check` | 通过或记录阻塞 |
| M7-A2 | workspace check | `just check` | 通过或记录阻塞 |
| M7-A3 | workspace test | `just test` | 通过或记录阻塞 |
| M7-A4 | provider 定向测试 | `cargo test -p agent-diva-providers vision` | vision request shape 通过 |
| M7-A5 | agent loop 定向测试 | `cargo test -p agent-diva-agent attachment` | 附件组装通过 |
| M7-A6 | GUI smoke | 启动 GUI，发送图片 + 文本 | vision 模型能基于图片回答 |
| M7-A7 | 日志归档 | summary / verification / release / acceptance | 满足 iteration-log-required |

建议负责人：当前交付负责人。

依赖：M1-M6。

## 6. 里程碑计划

| 里程碑 | 内容 | 退出条件 |
| --- | --- | --- |
| MM-PM | 项目管理计划完成 | 本文档和迭代日志完成 |
| MM-TYPE | 类型契约完成 | text-only 编译与测试不回归 |
| MM-SESSION | 会话附件完成 | 历史消息可保存附件元数据 |
| MM-AGENT | agent loop 图片组装完成 | prompt + 图片进入同一 user message |
| MM-CAP | capability guard 完成 | text-only 模型不会收到 image payload |
| MM-PROVIDER | OpenAI-compatible vision 完成 | 请求 JSON 包含 text + image_url |
| MM-GUI | GUI 最小体验完成 | 用户可发送图片并看到附件状态 |
| MM-ACCEPT | 验收完成 | 自动测试和 GUI smoke 记录完成 |

## 7. 执行顺序

推荐顺序：

1. MM-PM：冻结项目管理计划。
2. MM-TYPE：先做 provider message 类型契约。
3. MM-SESSION：补 `ChatMessage.attachments`。
4. MM-AGENT：agent loop 组装图片 part。
5. MM-CAP：加入 capability guard 和降级提示。
6. MM-PROVIDER：打通 OpenAI-compatible vision。
7. MM-GUI：做最小 GUI 显示和警告。
8. MM-ACCEPT：集中验证、记录和准备发布。

不建议并行过早拆分 M1-M5，因为类型契约、会话结构和 provider 序列化强相关。M6 可在 M2/M4 的接口稳定后并行。

## 8. 验收标准

### 8.1 产品验收

- 用户能在 GUI 附加 PNG/JPEG 图片并输入问题。
- 后端收到该图片的 file_id。
- vision-capable 模型请求中包含文本和图片内容块。
- assistant 回答能反映图片内容。
- text-only 模型不会收到 image payload，并给出清晰提示。
- reload session 后，用户历史消息仍显示图片附件元数据。

### 8.2 工程验收

- 旧文本消息和旧 session JSONL 兼容。
- provider `Message::user("text")` 等旧构造函数不破坏。
- OpenAI-compatible native provider 的 model ID 不被错误加前缀。
- 图片 bytes 不写入 session JSONL。
- 大图片、未知 MIME、file_id 不存在都有可解释错误。
- 文档与 `docs/logs` 完整。

### 8.3 验证命令

最小验证：

```powershell
just fmt-check
just check
just test
cargo test -p agent-diva-providers vision
cargo test -p agent-diva-agent attachment
```

GUI smoke：

```text
1. 启动 GUI。
2. 选择 vision-capable OpenAI-compatible provider/model。
3. 上传 PNG/JPEG。
4. 发送“这张图里是什么？”。
5. 确认 provider request 包含 structured image block。
6. 确认回答基于图片内容。
7. 重载会话，确认用户消息仍显示图片附件。
```

## 9. 风险台账

| 风险 | 影响 | 概率 | 控制措施 |
| --- | --- | --- | --- |
| 只改类型但链路未贯通 | 形成不可用半成品 | 中 | 每阶段必须有端到端验收点 |
| text-only provider 回归 | 现有聊天不可用 | 中 | 默认 `MessageContent::Text` 兼容路径 |
| 图片 bytes 写入历史 | session 膨胀、隐私风险 | 中 | 只保存 file_id 和 metadata |
| text + image 分成两条消息 | 模型无法正确理解上下文 | 中 | agent loop 必须组合为同一 user message |
| provider model ID 被误改 | DeepSeek 等 native endpoint 失败 | 中 | 加最终 outbound model 字段测试 |
| 大图片超限 | provider 拒绝请求 | 高 | 首阶段加大小限制，后续再做 resize |
| capability 判断不准 | 可用模型被拦截或不可用模型报错 | 中 | 默认保守，允许配置 override |
| GUI 乐观状态与后端历史不一致 | 用户体验混乱 | 中 | M6 只做最小标记，Phase A-PRE 后续强化 |
| MIME 识别不完整 | 图片被当普通文件 | 中 | 首阶段支持 PNG/JPEG/WebP，其他格式明确提示 |

## 10. 角色与责任

| 角色 | 责任 |
| --- | --- |
| 项目经理 | 控制范围、里程碑、风险、验收与日志完整性 |
| 后端负责人 | session、agent loop、manager 接口 |
| Provider 负责人 | message 类型、capability、OpenAI-compatible 序列化 |
| GUI 负责人 | 附件显示、发送路径、能力提示、smoke |
| QA / 验收负责人 | 自动测试、GUI smoke、回归记录 |

当前单人执行时，由当前交付负责人临时承担全部角色，但每个活动仍按上述责任域记录。

## 11. 变更控制

以下变更必须升级为单独评审，不直接塞进首阶段：

- 增加音频、视频、TTS 或图片生成。
- 增加 Anthropic / Gemini / Ollama 多 provider 全量 vision 支持。
- 改动 session canonical history 主架构。
- 重写 GUI 聊天主界面。
- 引入图片压缩/resize native 依赖。
- 改动 provider model ID 路由规则。

## 12. 下一步行动

完成本计划后，进入 MM-TYPE：

```text
在 agent-diva-providers/src/base.rs 中引入 MessageContent / MessageContentPart，
保留 Message::user("text") 兼容路径，
并为旧字符串 content 与新 parts content 添加序列化兼容测试。
```

建议第一轮开发 PR 或提交范围只覆盖 MM-TYPE，避免同时触碰 GUI、session 和 provider 请求发送路径。
