# Alife 能力取舍决策

- 原始记录：`docs/dev/archive(old-docs-dont-read-me)/2026-08-docs-corpus-reset/legacy-docs/root-history/DECISION.md`
- 日期：2026-06-18
- 状态：`Approved Direction / Historical Product Scope`
- 当前优先级：8/12–8/13 认知工作区架构记录优先；本文件保留产品方向，不直接授权实施。

## 决策摘要

| 能力 | 处置 | 备注 |
| --- | --- | --- |
| Live2D 桌宠 | 不做，或等 OLV2 后复评 | 版权与产品形态风险 |
| 深度视觉、OCR、屏幕识别 | 放入工作台 | 不塞入主对话流 |
| 语音对话 | 已有 | 由 Provider/transcription 承担 |
| 长期记忆 | 作为大版本能力 | 当前生产权威已由 BML/最新 Memory 决策重新定义 |
| 平台通讯 | 优先提升 channel 可靠性，新增通道 defer | 自动重连与异常回灌可作为参考 |
| 自主活动 | 下一生命周期重大功能 | 基于已完成基础设施，必须先完成架构设计 |
| Agent Browser | 放入工作台 | 与视觉同属重 UI/重工作流能力 |
| 脚本执行 | 已有 | 受 tools/sandbox 约束 |
| 多开互联/角色互聊 | 不做或继续 defer | 与单一人格连续性冲突 |
| 自我升级 | 重要，先调研 | 参考 GenericAgent、Hermes、Alife，不提前冻结模型 |
| 插件系统 | 与工作台一起设计 | 不单独制造第二产品面 |

## 自主活动方向

自主活动保留为下一生命周期能力，不等同于当前 Heartbeat、Cron、AutoDream 或 Evolution：

- Heartbeat 只负责触发条件；
- Rhythm 负责行为选择；
- Harmless/Safety 是横切护栏；
- Persona/世界/Memory/STM 的最终边界以 8/12–8/13 决策为准；
- 任何跨 session 意图持久化、自动沉淀或能力晋升都必须经过后续研究与架构门禁。

## 明确不做

- 不把多开互联作为当前核心方向；
- 不将视觉、浏览和插件各自发展为独立割裂的主产品；
- 不因为本决策中的历史术语恢复已被最新决策删除的旧人格、Memory 或治理链路。
