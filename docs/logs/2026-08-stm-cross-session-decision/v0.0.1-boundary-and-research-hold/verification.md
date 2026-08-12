# Verification

本次是只读代码/文档核对后的决策记录，没有运行产品代码测试或桌面 smoke。

已核实：

- 冻结的认知分区把 `05 MEMORY.MD` 定义为 STM 权威检查点与 Layer 1 有界 bootstrap；
- 当前 `MemoryMd/memory_md` 被当作 Long-Term Memory，存在 Persona、Evolution、Notebook、
  Proposal、Provider Prompt 和迁移路径；
- BML typed SQLite/FTS5 已是生产长期 Memory 权威；
- 当前 `WorkingMemory` 是 per-session volatile checkpoint，session end 会物理删除；
- GUI MemoryView 只能把 `working_memory` 当普通 BML kind 筛选，没有统一 STM 工作区；
- Persona 左栏仍把 `memory_md` 放入 `long_term` 分组。

因此当前问题同时包含领域双权威、生命周期命名冲突和 GUI 信息架构缺失。实施前仍须完成
专项调研、保护性分支、删除证明、完整自动化测试和真实桌面跨会话验收。
