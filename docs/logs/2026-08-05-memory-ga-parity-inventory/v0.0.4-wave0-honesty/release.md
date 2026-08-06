# Release — GA-MEM-PARITY Wave 0（诚实与契约）

## 发布方式

代码变更随常规 crate 构建发布（core/laputa/agent/manager），无独立部署物。

## 行为变更提示

- **出箱默认 authority 改为 Typed**（W0-B）：已有 config 若缺失 `memory` 段，
  升级后默认走 Typed SQLite 权威；显式 `authority_mode: "legacy"` 不受影响。
- sync_turn 返回语义更诚实：proposal 路径上报 `ProposalCreated`（此前
  `Persisted`）；依赖「Persisted=权威生效」语义的下游需确认（已核实消费方
  仅 consolidation，语义兼容）。

## GUI/CLI 影响

- 无 wire/SSE/Tauri 协议变更（SyncTurnStatus 与 config 默认不进协议）。
- GUI EvolutionView 仅展示 authority_mode 文本，无需改动。
