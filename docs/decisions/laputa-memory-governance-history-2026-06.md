# Laputa / Memory 治理历史决策

- 原始记录：归档批次 `2026-08-docs-corpus-reset/legacy-docs/prds/content/prd-laputa-2026-06-12/.decision-log.md`
- 日期：2026-06-12 至 2026-07-08
- 状态：`Historical Foundation / Superseded by BML–Persona–STM Clean Break`

## 保留的真实决策

- Laputa 负责人格/治理方向，Memory 负责存储与读取；提案、用户审查、审计和回滚不能被普通记忆写入绕过。
- 早期“多文件人格”“Memory.md”“Mentle/Laputa 混合层”等方案经过多轮评审后逐步收敛为更薄的文档/治理边界；这些方案不是永远有效的实现合同。
- 任何自主进化写入人格或长期记忆，都必须先经过治理边界和用户可见的审查路径。
- 迁移、恢复、冲突和失败必须保留可观察结果，不能用静默 fallback 掩盖数据或治理状态变化。

## 当前关系

2026-08 的最新架构已将生产长期记忆统一到 BML，并将 Persona、STM、Evolution 与 Chat Approval 分成认知工作区；旧 Laputa/Mentle 物理实现、提案链路和文件布局不得从本文恢复。当前入口是 `docs/architecture/current/cognitive-workspace-boundaries-2026-08.md`、`docs/research/bml-layer-extraction-2026-08/`、`docs/research/persona-markdown-clean-break-2026-08/` 和 `docs/research/stm-cross-session-clean-break-2026-08/`。
