# 压缩技术调研完成总结

## 版本
v0.0.1-compression-design

## 日期
2026-05-30

## 变更内容

完成 Agent-Diva autodream 前置压缩技术调研，产出 `docs/dev/genericagent/compression-research.md`。

核心结论：

1. **现状诊断**：当前 `consolidation.rs` 是"会话段总结器"，不是"可审计压缩器"。它直接通过 `MemoryProvider::sync_turn()` 写入 MEMORY.md，没有结构化产物、没有证据引用、没有候选审查、没有重跑能力。

2. **设计方向**：引入 Source Capsule 作为压缩中间产物，存放于 `.agent-diva/compact/capsules/*.json`。压缩是 autodream 的材料准备层，不是长期记忆写入层。

3. **边界划定**：
   - 压缩不直接改写 L2/L3/SOUL/SOP
   - 压缩产物供 autodream 消费，不注入日常 prompt
   - 原始 evidence 保留不动
   - 写入长期记忆需经过 LearningCandidate/用户确认

4. **MVP 定义**：Phase 1 只做 session-segment compact → source capsule 写入 → 事件记录。后续再加 autodream 前置扫描和 capsule 合并。

## 影响范围

- 不改动任何代码，纯调研文档
- 更新 `docs/dev/genericagent/README.md` 索引
- 后续实现时影响 `agent-diva-agent/src/`、`agent-diva-core/src/session/`

## 变更文件

- 新增：`docs/dev/genericagent/compression-research.md`
- 修改：`docs/dev/genericagent/README.md`（新增索引条目）
