# S5 复核修复：活合同纠偏

- 日期：2026-08-16
- 切片：S5 review live-contract repairs（P0-A / P0-B / P1-C / P2-D）
- 分支：`agent-diva-pro`（未 push）
- 版本目录：`docs/logs/2026-08-s5-review-repairs/v0.0.1-live-contract-repairs/`

## 目标

独立复核 `710e7684`..`fc96377c` 后，只修确认的活合同错误。不回退 S5 卸旧，不进 S6，不清 `put_governed` schema。

## 提交

| Commit | 内容 |
| --- | --- |
| `0c1f0dfd` | WORLD 形巩固项丢弃，不再 `memory_add` 进 BML |
| `fb1961e5` | 系统 Prompt 不再教「改/删记忆要审批」 |
| `3e9631bc` | `ContextBuilder::new` / `with_skills` 隔离到传入 workspace |
| `c011e680` | Approval / capability / CLI 去掉 memory 审批域；删除 `propose_section_write` |

## 明确未做

- S6 `just cognitive-clean-break-check` 与桌面 smoke
- `put_governed` / `rollback_governed` 代码路径删除（schema 必须留）
- `MemoryCrudOutcome::ProposalCreated` 枚举删除
- 不给 consolidation 补 `persona_request(WORLD)` 写核
