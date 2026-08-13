# Reconcile — autodream PRD v1.1 + design spec vs selfinprove PRD v0.0.4

> Input reconciliation (bmad-prd Finalize Step 2). 本文仅做 gap 标注, 不写实现细节, 关注能力级 / 边界 / 约束。
> 材料 A: `prd-autodream-2026-06-12/prd.md` v1.1 (381 行, 14 FR, scope-merge 修订)
> 材料 B: `agent-diva-selfinprove/docs/dev/genericagent/autodream-rhythm-distillation-design.md` (893 行, P0a/P0b/P1/P2 分阶段)
> 待检: `prd-selfinprove-2026-06-12/prd.md` v0.0.4 + `.decision-log.md`

## Summary

selfinprove PRD v0.0.4 在 Vision / Scope / FR-1xx 已经把"消费 AutoDream 候选 + 写 Laputa" 主链路搭起来了, 但跟 autodream PRD v1.1 + design spec 比对, 还存在 8 类缺口。最严重的是 trigger 同步性矛盾 (selfinprove 假设 manual 同步, autodream PRD FR-1 是 task ID 轮询异步) 与 phase 错位 (selfinprove 假设 cron 异步后台跑是主路径, design spec 把这放在 P1, P0a 仅 manual)。AutoDream v1.1 内部还有 3 处已知瑕疵 (FR-7 Orient 只读 Laputa / FR-11 写 MEMORY.md vs Non-Goals §5 矛盾 / FR-12/13 划给 Report System), 这些都不直接影响 selfinprove 能力, 但 selfinprove 在写 Laputa 路径、changelog 共享、evidence 锚点上都缺显式 back-link, 会导致后续接 Laputa PRD 时二次返工。Mentle 集成 / 4 阶段 prompt / trigger_kind 枚举 / auto_mode 默认 off 提醒 / graceful degradation 标记 在 selfinprove PRD 里完全没提, 属于"假设上游已就绪"型盲点。

## Gap Table

| # | 缺口 | 来源 (autodream 侧) | selfinprove 侧表现 | 严重度 |
|---|------|---------------------|---------------------|--------|
| G1 | manual trigger 同步性矛盾 | PRD FR-1 "返回 task ID 供轮询" / design §6.2 manual "Blocks until complete or user cancels" | FR-302 写 "Chat 命令 + 卡片按钮, 同步调用", D-009 也按同步假设 | 高 |
| G2 | cron 异步后台跑 phase 错位 | design §15 P0a=manual / P0b=eligibility signal only / P1=cron 异步 / P2=full rhythm | Vision 段把 "AutoDream 后台跑" 当默认主路径, §4 Scope 表把"cron 异步"当 IN | 高 |
| G3 | auto_mode 默认 off UX 缺失 | PRD FR-2 "auto_mode_enabled toggle 默认 false" | FR-301 仅写"配置 daily/weekly/manual", 没标"auto_mode 默认 off, 用户不开就不跑" | 中 |
| G4 | trigger_kind 枚举未透出到 diagnostics | design §9.1 trigger.kind 枚举 (manual / session_end / time_gate / session_count_gate / startup_catchup) / PRD FR-9 事件流含 trigger_type | FR-303 diagnostics 5 字段无 trigger_type, 用户无法判断"这次为何跑" | 中 |
| G5 | Changelog 路径共享未声明 | design §10.4 "memory/changelog.jsonl (共享 across all MEMORY.md mutation sources)" | FR-106 / FR-204 / FR-602 反复"走 changelog", 但未指明路径, 也未声明与 AutoDream 共享还是分账 | 中 |
| G6 | Laputa 写权限分配不清 | design §10.3 Critical/Sensitive tier "→ 写 .laputa/inbox/learning-candidates.jsonl" + §11.2 JournalEntry 写 `.laputa/rhythm/` | FR-501 "POST /api/laputa/proposals/{id}/apply" 走统一 API, 没声明与 AutoDream 写 inbox/learning-candidates.jsonl 的分工 | 高 |
| G7 | evidence 锚点用错字段 | design §9.1 evidence_refs 含 session_id + turn_index + excerpt_hash | FR-107 "经 evidence_excerpt 锚定的 session_id" 用摘要做锚点, 无法精确回滚到具体 turn | 中 |
| G8 | Mentle 集成决策缺失 | PRD Non-Goals §5 "v1 不支持 Mentle 召回 (P1)" / design §8.1 priority 5 列 Mentle / design §5.3 HybridMemoryProvider Mentle-first-then-Markdown | selfinprove PRD 完全未提 Mentle, 写 Laputa 是否要等 Mentle 同步未决策 | 低 (P1 风险) |
| G9 | 4 阶段 prompt 无 back-link | design §4.5 Orient→Gather→Consolidate→Prune/Index / PRD FR-7 改名 Propose | selfinprove 不重复定义 4 阶段合理, 但缺 1 句 back-link 说"信任 AutoDream 产物 schema" | 低 |
| G10 | graceful degradation 标记未透出 | design §13.2 "mark degraded:true + degradation_reason, checkpoint 仍前进" | FR-303 diagnostics 无 degraded 字段, 用户不知"这次产物是否完整" | 低 |
| G11 | lock 冲突时 UI 行为未定义 | PRD FR-3 lock 机制 / design §6.2 manual "still checks lock" | FR-302 manual 触发撞到 lock 时, 是排队 / 报错 / 自动清 stale 都没写 | 中 |
| G12 | FR-11 vs Non-Goals §5 内部矛盾 | PRD FR-11 "接受的候选写入 MEMORY.md" vs Non-Goals §5 "不直接写 MEMORY.md" | selfinprove FR-103 "应用 → 写 target_file" 实际接管写权限, 但未 back-link 说明 | 中 |

## Actionable (8 条)

1. (对接 G1) 在 autodream PRD 新增 FR-15 "in-session sync manual trigger", 明确 sync 调用契约 (返回 run_id + progress stream, 不走 task ID 轮询); 或 selfinprove FR-302 改写为 "Chat 命令 → 卡片轮询 task ID, 进度条驱动", 跟 FR-1 对齐。Decision log D-009 的"同步调用"假设需更新。

2. (对接 G2) selfinprove Vision 段加 1 句约束: "AutoDream cron 异步路径依赖 design spec P1 ship; P0a 仅 manual 可用, Vision 主路径在 P1 前以 manual 触发为 fallback"。Scope 表 "cron 异步" 行加 phase 标注 "[AUTO-AVAIL-AFTER-AutoDream-P1]"。

3. (对接 G6) selfinprove §4 Scope 表加 "Laputa 写权限分配" 一行: AutoDream 写 `.laputa/inbox/learning-candidates.jsonl` (候选入站, P0a/P1 均有) + `.laputa/rhythm/daily/` (journal_entry, P1 起); Selfinprove 走统一 Laputa API (`POST /api/laputa/proposals/{id}/apply`) 写最终目标文件 (identity / preference / relationship / project / fact), 不直接动 inbox 目录。Design §10.3 + §11.2 back-link。

4. (对接 G5) selfinprove FR-106 / FR-204 / FR-602 加 1 句 "changelog 路径: `.agent-diva/memory/changelog.jsonl`, 与 AutoDream 共享 (per design §10.4)"; §3 Glossary 加 MemoryChangelog 词条; FR-605 "30 天内可回滚" 改写为 "changelog 保留期 ≥ 30 天, 与 AutoDream 共享保留策略"。

5. (对接 G3 + G4 + G10) selfinprove FR-303 EvolutionRunDiagnostics 字段从 5 个扩到 8 个, 加 trigger_type (枚举: manual / session_end / time_gate / session_count_gate / startup_catchup, 透出 AutoDreamEvent.EnrichedTrigger) + auto_mode_enabled (布尔, 默认 false, 用于提示用户"cron 没开") + degraded (布尔 + degradation_reason 字符串)。FR-301 配置面板加 1 行提示: "auto_mode 默认关闭, 关闭时仅响应 manual trigger"。

6. (对接 G7) selfinprove FR-107 跨表面跳转锚点改写: "经 evidence_refs[].session_id + turn_index 锚定原始 session turn, evidence_excerpt 仅作预览"; 新增 FR-110 "evidence 回溯面板", 在 Inbox 详情页可点击跳到 evidence_refs 指向的具体 session turn (而非 session 概览)。

7. (对接 G8 + G9) selfinprove §3 Glossary 加 Mentle 词条 + 1 句 "v1 不集成 Mentle (per autodream Non-Goals §5), P1 集成时再决策 Selfinprove 写 Laputa 是否同步 Mentle"; §5 加 FR-901 [outline] "Mentle 同步决策", 等 Laputa PRD 出。Vision 段加 1 句 back-link: "AutoDream 4 阶段蒸馏 (Orient → Gather → Consolidate → Propose, per design §4.5 + PRD FR-7) 产出 schema 在 `autodream_run.json`, Selfinprove 信任该 schema 不重复解析 4 阶段"。

8. (对接 G11 + G12) selfinprove §6 Acceptance / FR-302 加 "manual 触发撞到 lock 时的 UI 行为契约": (a) lock 被活进程占 → 卡片显示"AutoDream 已在跑 (PID xxx, 启动 N 分钟前), 等结束 / 强制取消"; (b) stale lock (>60min) → 卡片自动回收并提示用户; (c) FR-103 应用 → 写 target_file (实际接管 autodream PRD FR-11 的写权限, 跟 Non-Goals §5 不矛盾, 因 Selfinprove 不属于 AutoDream 子代理), Decision log 加 D-012 "写权限归属澄清"。

---

## 附录: 不属本轮 reconcile 但需登记的盲点

- **PRD v1.1 内部瑕疵** (autodream PRD 自身问题, 不影响 selfinprove 能力但下游需知): FR-7 Orient 只读 Laputa 已隐含跟 §10 边界一致, OK; FR-11 vs Non-Goals §5 矛盾已通过 Selfinprove 接管写权限消解, 见 actionable #8; FR-12/13 日报/周报生成被 scope-merge 划给 Report System, selfinprove Vision 段"跨日报/周报追踪人格漂移"实际指 Inbox 显示的 journal_entries, 跟 Report System 的 daily/weekly 报告不重叠, 但需在 §1 加澄清避免读者混淆。
- **Sibling 文档断链** (D-004): autodream PRD 第 17-21 行 5 处引用 docs/dev/genericagent/* 全缺失, D-004 已登记 repair 工作, 本 reconcile 不重复处理。