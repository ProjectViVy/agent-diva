# E3 Reflection Engine 与 Candidate Gate

本切片移除 AutoDream worker 中固定 `journal_note` 占位候选，接入真实的
provider-neutral Reflection 契约与生产 provider adapter。

- 生产反思复用 Manager 现有 provider resolver，保持原始 provider model ID。
- 模型仅接收有界 schema、PII 脱敏证据和既有 Memory digest；既有 Memory 原文仅在
  本地 gate 使用，不发送给 provider。
- `MemoryCandidate` 包含 proposal type、内容、evidence、confidence、scope、
  sensitivity、expected value 和 invalidation conditions。
- Candidate Gate 验证完整 evidence ref，并拒绝 secondary-only、伪造证据、重复、
  直接矛盾、低置信度、低价值、越 workspace、越容量、prompt injection 和敏感内容。
- 已延期的独立 SOP 类型不允许由 AutoDream 生成；未来流程编辑统一走 Skill 架构。
- accepted candidate 的完整元数据保存在 AutoDream artifact；rejection 仅记录
  candidate ID 与稳定 reason code，不记录内容。
- provider 不可用/超时/失败/非法 schema 使用稳定失败码；空候选是合法、诚实的
  `no_candidates` 完成结果。

AutoDream 仍只能创建待审 Laputa proposal，不能直接写 Memory 或自行批准。
