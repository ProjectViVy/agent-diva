# 当前实现、参考项目与标准证据

## 1. agent-diva 当前基础

### Tool 投影

- `agent-diva-tooling/src/base.rs` 的 `ToolRegistry` 已有注册、注销和统一执行；
- `agent-diva-tools/src/mcp_sdk.rs` 已将 MCP Tool 包装为原生 Tool；
- `agent-diva-agent/src/tool_assembly.rs` 会把 MCP 工具装入 Agent 使用的 registry；
- `agent-diva-agent/src/agent_loop.rs` 可按 turn 重建工具集合。

这些路径证明“外部能力经策略投影为 DIVA 原生 Tool”是可行的。PEN 不需要重新发明
Agent Tool 调用协议。

### 运行时与治理

- Manager 已有 HTTP 控制面和受监督任务；
- RunStore/Executor 已有排队、运行、完成、失败、取消和恢复基础；
- Sandbox 已有 Windows/Linux/macOS 平台适配与审批；
- Approval/Audit 可成为模块调用的统一治理面；
- Laputa/BML 已有明确写入边界，外部模块不得直写。

### 当前缺口

- ChannelHandler 只覆盖 start/stop/send/allowlist 等最小能力；
- MessageBus 仍有无界队列，缺少统一背压；
- 模块 Bootstrap/ModuleRegistry 尚未成为 Manager 的生产统一入口；
- MCP 当前主要消费 Tools，尚未完整覆盖 Resources/Prompts 等；
- 缺少 Package/Instance/Publisher/ExternalEntity 身份；
- 缺少第三方模块 Supervisor、Trust Store、Secret Broker 和 Capability Snapshot。

## 2. Neuro-Link 当前原型风险

当前 `agent-diva-channels/src/neuro_link.rs` 是简化的本地 WebSocket pipe：

- register 和 sender/role 主要由客户端自报；
- 缺少协议版本和挑战认证；
- 缺少能力协商、ACK、背压、取消、恢复和多路复用；
- 当前配置默认 host 与“限制本地访问”的注释存在矛盾；
- Avatar 使用特殊 chat ID 和 `speak` metadata；
- 连接/聊天 sink 模型不足以承载完整多前端状态同步。

这不否定 Neuro-Link 方向，只说明现有代码是概念胚胎，不能以增加几个 PipeMsg variant
代替重量级前端合同设计。

## 3. 当前 Mate 是 Mirror 胚胎

`agent-diva-gui/src/features/diva-mate/` 已包含：

- Workbench 内嵌 Mate View；
- 独立 DesktopMate Overlay；
- VRM 模型、动画、表情和外观；
- 语音输入、ASR/TTS、字幕；
- 当前 GUI 的设置和资源管理。

`agent-diva-manager/src/runtime/task_runtime.rs` 又通过 Neuro-Link Avatar chat ID 转发
`speak`。这证明 Mirror 与 Neuro-Link 已存在真实需求，也暴露了 GUI、Manager、Channel
和 Avatar 语义相互渗透的问题。

## 4. 旧工作台资料的保留与修订

本轮读取的外部原始资料为
`C:\Users\Administrator\Desktop\morediva\00-创意工作台设计\README.md`。该目录不属于
本仓库，因此本研究包保存的是经过当前 DIVA 权威边界校正后的结论，不把外部文件作为
运行时或发布依赖。

外部工作台旧稿提出“Microkernel + 插件宇宙”“能外接就外接”“主人通道与群聊拥有不同
信任/权限/记忆语义”，方向仍然有效。

需要修订：

- 历史 Mentle 统一为 BML 存储 + Laputa 治理；
- “按需编译”改为“按需安装、按需启动、按上下文授权”；
- “桌面图标 = 已安装能力”只保留为视觉隐喻，不作为数据模型；
- Workbench 不替代已批准的 Persona/Memory/Evolution 等领域工作区。

本仓现有方向决策也已将深度视觉、OCR、屏幕识别、浏览器和插件管理归入工作台，并要求
不要把这些重 UI/重工作流能力塞入主对话流：

- [产品范围与工作台边界](../../decisions/product-scope-and-workbench-2026-06-18.md)
- [Alife 能力取舍](../../decisions/alife-feature-disposition-2026-06-18.md)

## 5. OpenFang Hands

本轮本地快照为 `.workspace/openfang@acf2587e46be`。其中 Hands 是由 `HAND.toml`、
Skill、Tool、MCP、Guardrail 和
Schedule 组成的自治应用包。它适合参考：

- 能力/技能/配置一起打包；
- 激活后生成受管理 Agent；
- Manifest 和签名；
- 浏览器能力与应用模板组合。

Hands 不等于 PEN：

- Hand 更接近“自治 Agent 应用包”；
- PEN 是能力设备/外部实体边界，不要求自治；
- OpenFang 浏览器能力仍主要内置于其 Runtime；
- 只有内嵌公钥的签名验证不足以构成完整发布者信任链；
- 空 allowlist 解释必须按 DIVA 规则 fail closed。

未来 Hand 类应用可以建立在 WorkbenchModule/PEN 底座之上，但不把其 Runtime 整体嵌入
DIVA Core。

## 6. Hermes

本轮本地快照为 `.workspace/hermes-agent@cf328723d43d`。其插件系统可注册 Tool、
Platform、Browser Provider 和
Hook，并支持发现和重新扫描。适合参考：

- Manifest 与插件发现；
- Provider registration；
- 原生 Tool projection；
- 平台/浏览器实现可替换；
- reload 用户体验。

但 Hermes Python 插件通常直接导入宿主进程，不构成 DIVA 所需的强隔离。其 Browser
Provider 主要是浏览器 Session 后端，也不是完整外部实体 Runtime。DIVA 应借注册语义，
不照搬信任模型。

## 7. A2A 与频道研究的依赖

本研究不重写已经完成的两个研究包：

- [A2A 互操作研究](../a2a-interoperability-2026-08/README.md)
- [外部频道能力对照](../channel-capability-reference-2026-08/README.md)

A2A、普通 Channel、Neuro-Link 和 PEN 应共享 message ID、typed parts、task/run、取消、
流、receipt、身份和审计基础，但各自保留协议语义。

## 8. 外部标准

### A2A

[A2A v1.0 规范](https://a2a-protocol.org/latest/specification/)定义 Agent Card、接口
能力、Message、Task、Artifact、流式更新和取消。DIVA 可将远程 Agent 建模为
External Entity + AgentExport，但不能把 A2A 当作本地模块生命周期协议。

### MCP

[MCP 2025-06-18 规范](https://modelcontextprotocol.io/specification/2025-06-18/index)
将协议分为生命周期、鉴权和客户端/服务端能力；
[Server 能力说明](https://modelcontextprotocol.io/specification/2025-06-18/server/index)
区分 Tools、Resources 和 Prompts。当前 DIVA 的 Tool 投影是良好起点，但 PEN 的 MCP
Driver 以后不应永久丢失 Resources/Prompts 等语义。

### WebAssembly Component Model

[WIT](https://component-model.bytecodealliance.org/design/wit.html)适合描述类型化模块
接口；[WASI 0.3 FAQ](https://component-model.bytecodealliance.org/reference/faq.html)
记录其异步函数、stream 和 future 进展。它适合作为后续受限 Runtime 选项，但版本较新，
不应阻塞 process/stdio MVP，也不能未经验证提高当前 Rust 1.80 MSRV。

### OCI 与 Sigstore

[OCI Image Spec](https://specs.opencontainers.org/image-spec/)可作为以后内容寻址、
多平台包和分发的参考；[Sigstore Cosign 验证](https://docs.sigstore.dev/cosign/verifying/verify/)
适合发布者身份、签名和证明。MVP 可以先使用本地信任根和 digest，但 Manifest 必须为
后续签名链预留字段。

## 9. 综合判断

PEN 的独特性不在于发明新的 RPC，而在于把包、独立运行时、外部实体、能力投影、
上下文授权、工作台和 DIVA 治理组合为一个完整产品模型。当前不能严谨宣称全球首创；
只有在形成规范、威胁模型、TCK 和安全实现后，才会成为可验证的 DIVA 特性。
