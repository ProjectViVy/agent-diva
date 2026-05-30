# 压缩技术调研验收

## 版本
v0.0.1-compression-design

## 验收标准与结果

### 核心问题回答

| 问题 | 回答位置 | 结果 |
|---|---|---|
| 当前 consolidation 是"会话段总结"还是"可审计压缩"？ | §2.4 差距分析 | PASS：明确诊断为"会话段总结器"，列出 7 项差距 |
| 压缩输入有哪些？ | §4.2 + §5.1 | PASS：session messages, tool results, plan evidence, history |
| 最小压缩产物应该长什么样？ | §5.1 Capsule Schema | PASS：包含 summary/key_facts/decisions/open_threads/candidate_lessons/evidence_refs/source_session_ids/created_at |
| 压缩结果放哪里？ | §5.4 | PASS：`.agent-diva/compact/capsules/*.json` + `events.jsonl` |
| 压缩什么时候触发？ | §6.1 | PASS：turn-end / session-end / manual / autodream 前置 |
| 压缩和 autodream 的边界？ | §7.2 | PASS：压缩产出 capsule，autodream 消费 capsule 做 rhythm/候选 |
| 压缩和长期记忆写入的边界？ | §7.1 + §7.5 | PASS：不直接改 L2/L3，经 LearningCandidate 确认 |

### MVP 定义

| MVP 要求 | 结果 |
|---|---|
| 先做什么 | PASS：Phase 1 session-segment compact → capsule → events（§8.1） |
| 不做什么 | PASS：不直接改 MEMORY/SOUL、不调 Mentle、不做跨 session 去重（§8.5） |
| capsule schema 草案 | PASS：§5.1 完整 JSON schema |
| 触发/checkpoint 方案 | PASS：§6 四触发点 + checkpoint 设计 |
| 失败不影响主 session | PASS：§8.7 风险表 + §6.3 checkpoint 规则 |
| 哪些写入必须经过确认 | PASS：§7.5 确认要求表 |

### 参考架构对齐

| 对齐项 | 结果 |
|---|---|
| 不绕过 MemoryProvider | PASS：§7.1 明确禁止 |
| 不污染 .laputa/ | PASS：capsule 存 .agent-diva/compact/ |
| 原始 evidence 保留 | PASS：§1 核心原则 |
| 支持 Journal 审计 | PASS：§7.3 通过 evidence_refs 追溯 |
| 在线路径保持轻量 | PASS：§4.2 触发点设计 |

## 验收结论

PASS。调研报告完整回答了所有要求的问题，MVP 定义清晰，边界划定与 architecture.md 和 ui-design.md 一致。
