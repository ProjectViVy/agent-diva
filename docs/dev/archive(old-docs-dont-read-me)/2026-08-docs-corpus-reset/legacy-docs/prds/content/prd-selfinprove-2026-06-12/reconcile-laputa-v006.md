# Reconcile: Laputa PRD v0.0.6 (final) vs selfinprove PRD v0.0.1 (draft)

> bmad-prd Finalize Step 2 (Input reconciliation) — John (PM) 2026-06-12
> 取代子 agent IR-1 (failed @ max_iterations), 我自己跑

---

## Summary (5-8 句)

1. Laputa PRD v0.0.6 final, 36 FRs 已固化 (写/读/事件/changelog/14schema/迁移/Mentle 边界/NFR + FR-507/508/509 端到端)
2. selfinprove PRD v0.0.1 draft, 仅有 FR-1xx 写接口 9 条 (FR-101~109), FR-2xx~7xx 全是 outline (per v0.0.1 现状)
3. selfinprove FR-5xx (FR-501/502/503/203) 标 `[LAPUTA-STUB]`, 现在 Laputa 是 final, **stub 应实质化** (调用 FR-1xx~4xx + FR-5xx 实际接口, 不再 mock)
4. selfinprove §3 Concerns 只有 8 条 (C-1~C-8), 缺 Laputa PRD D-010 触发的 scope 放大器 (Journal 批处理)
5. selfinprove §6 Open Questions 仍空白 (v0.0.1 留 stub), 9 条来自 D-004 + 后续 reconcile 待填
6. selfinprove §7/§8 缺 (Success Metrics / Glossary)
7. selfinprove D-010 悬而未决 (Laputa 写权限依赖问题), Laputa PRD finalize 后实质是 "一锅端" (B 选项), 应回写 D-010 关闭
8. selfinprove 缺 Laputa PRD 拆 stub 的 Sprint Change Proposal (Laputa D-013 已 flag)

## Gap Table (8 行)

| # | selfinprove 段 | Laputa 对应 | 状态 | 备注 |
| -- | -- | -- | -- | -- |
| G1 | §5 FR-2xx 固化流程 (outline) | Laputa 写接口 + changelog (FR-101/103/403/501) | ❌ missing | selfinprove 需把 outline 扩成 full FRs, 引用 Laputa FR-501 拆 stub |
| G2 | §5 FR-3xx 节律/触发/诊断 (outline) | Laputa 订阅 + D-009 节律双模 | ❌ missing | selfinprove FR-301 manual trigger in-session, FR-303 diagnostics, 缺节律配置 UI |
| G3 | §5 FR-4xx Changelog 路由 (outline) | Laputa FR-4xx (list/detail/rollback) | ❌ missing | selfinprove 需 4xx 全本, 接 Laputa 真实路由 |
| G4 | §5 FR-5xx 14 文件 schema + 迁移 (outline) | Laputa FR-5xx (14 section + 旧 8 → 1 稀薄) | ❌ missing | selfinprove 需引用 Laputa §4.1 14 份分类表, 不重写 |
| G5 | §5 FR-6xx Mentle 边界 (outline) | Laputa FR-6xx 3 条已 full | partial | selfinprove 需拉 Laputa 实际 3 条, 标 selfinprove 侧契约 |
| G6 | §5 FR-7xx NFR (outline) | Laputa FR-7xx 6 条已 full | partial | selfinprove 需把 NFR 适配 selfinprove 上下文 (不是 Laputa) |
| G7 | §6 Open Questions 空白 | Laputa D-008 OQ 12-20 + selfinprove D-004 OQ 1-9 | ❌ missing | 需 populate, 标 Laputa 后已关 / 转 follow-up / 待 polish |
| G8 | D-010 悬而未决 (Laputa 写权限依赖) | Laputa PRD finalize 后实质 B 一锅端 | ⚠️ stale | 应回写 D-010 关闭 + 推 Sprint Change Proposal 拆 stub |

## Actionable (8 条)

1. **扩 FR-2xx 固化全本** (5 条 → full, 引用 Laputa FR-501 拆 stub, per G1)
2. **扩 FR-3xx 节律/诊断全本** (5-7 条 → full, 引用 Laputa FR-301/303/603, per G2+G5)
3. **扩 FR-4xx Changelog 路由全本** (3-5 条 → full, 引用 Laputa FR-401/402/403, per G3)
4. **扩 FR-5xx 14 schema + 迁移全本** (4-5 条 → full, 引用 Laputa FR-501/502/503, per G4)
5. **拉 Laputa FR-6xx Mentle 3 条到 selfinprove** (per G5)
6. **拉 Laputa FR-7xx NFR 6 条, 适配 selfinprove 上下文** (per G6)
7. **populate §6 Open Questions** (per G7, 至少 11-15 条, 标 Laputa 后状态)
8. **回写 D-010 关闭 + 推 Sprint Change Proposal 拆 stub** (per G8)
