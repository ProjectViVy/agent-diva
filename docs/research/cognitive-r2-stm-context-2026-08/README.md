# COGNITIVE-R2：STM 与上下文分层专项研究

- 状态：`Research Complete (package) / Research Gate Pending User Review`
- 记录日期：2026-08-13
- EPIC：`LAPUTA-COGNITIVE-WORKSPACE-RESET` → **R2**
- 性质：专项研究事实、选项与约束；**不授权**目标架构定稿或代码实施
- 依赖：全量 R0 [`../cognitive-r0-current-state-2026-08/README.md`](../cognitive-r0-current-state-2026-08/README.md)；
  R1 L0–L4 [`../cognitive-r1-genericagent-evolution-2026-08/l0-l4-memory-architecture.md`](../cognitive-r1-genericagent-evolution-2026-08/l0-l4-memory-architecture.md)；
  产品边界 [`../stm-cross-session-clean-break-2026-08/decision-record.md`](../stm-cross-session-clean-break-2026-08/decision-record.md)

## 一句话结论

产品 STM（workspace/profile 级、自动维护、有界、跨 session 持续的活动工作集）**在代码中不存在**。
当前被误读为 STM 的是三条寿命不同的链路：session JSONL 里的 `canonical_checkpoint_v1`、BML
`WorkingMemory` session scratch，以及 Typed L1 长期 Memory 索引。R2 把它们拆开，写清 C1–C5
装配约束和失败矩阵，并比较 Research Hold 选项；**不选物理权威，不定 D2 方案**。

## 阅读顺序

1. [context-assembly-constraints.md](./context-assembly-constraints.md) — 当前对象图、C1–C5 装配、Session / Plan / subagent / cron 生命周期、Layer 1 可插入位置约束
2. [stm-options-and-experiments.md](./stm-options-and-experiments.md) — Research Hold 逐条选项、静态实验、晋升证据边界
3. [stm-failure-and-concurrency-matrix.md](./stm-failure-and-concurrency-matrix.md) — 新 session / resume / 多 channel / 崩溃 / 多进程 需求与缺口

## EPIC 完成物映射

| EPIC 要求 | 本包文件 |
| --- | --- |
| `stm-options-and-experiments.md` | 同名 |
| `context-assembly-constraints.md` | 同名 |
| `stm-failure-and-concurrency-matrix.md` | 同名 |

## 证据分级

| 标签 | 含义 |
| --- | --- |
| 源码事实 | 当前树可定位实现 |
| 提交事实 | git / 已落地决策记录 |
| 实验观察 | 本包静态 `rg` / 对照测试阅读；无新桌面复测、无活体 LLM |
| 推断 | 由事实推导 |
| 建议 | 研究标记，非架构批准 |

## 产品约束（研究不得推翻）

- STM 跨 session 持续；不是长期 Memory、transcript、`canonical_checkpoint` 或现有 `working_memory`
- Memory CRUD 与 STM 日常维护不走审批；BML 是普通长期 Memory 唯一权威
- `memory_md` / `MemoryMd` 属于 clean-break 删除范围，不能当 STM 落点
- session checkpoint 结束清理不得删除 STM；STM 清理不得删除 BML 或 transcript
- STM 不得把推测直接晋升为 BML 长期权威
- 入口只属于 Memory 工作区

## 研究 Gate 自检

- [x] 三份 EPIC 完成物齐全
- [x] 引用源码路径与符号可复现
- [x] 结论区分源码事实 / 提交事实 / 实验观察 / 推断 / 建议
- [x] 现有三链路与产品 STM 的边界无未声明重叠
- [x] Research Hold 全部出现且未默认选择
- [x] 静态实验可复现；活体缺口显式列出
- [x] R0 / R1 被引用，不重写成第二份权威
- [ ] 用户 Research Gate 评审（待）

## 明确不做

目标 DTO/schema、STM 物理权威定稿、Layer 1 装配位置定稿、GUI 线框、
`run_startup_gc` 是否接线、保护性分支、生产代码修改。

## 三套不得再混名的对象

| 本包称呼 | 当前代码对象 | 产品身份 |
| --- | --- | --- |
| SessionCheckpoint | `MemoryRecordKind::WorkingMemory` + `update_working_checkpoint` | session scratch；S4 要求改名，≠ STM |
| CanonicalCheckpoint | `Session.canonical_checkpoint` / `canonical_checkpoint_v1` | 对话压缩检查点；KEEP，≠ STM |
| BmlStartupIndex | `ContextSection::MemoryPolicyAndIndex` / `render_l1_index_block` | 长期 Memory 有界索引；≠ STM |
| 产品 STM | **不存在** | S3 活动工作集；物理权威 Research Hold |

另有一个历史同名陷阱：GenericAgent **L1** = insight 存在性索引；Diva Wave 2 **L1** = BML
记录预览；Garden **Layer 1** = STM bootstrap。只有第三项接近 S3。见 R1 L0–L4 与本包选项文。

## 开放缺口（交给后续）

| 缺口 | 交给 |
| --- | --- |
| 选 STM 物理权威、scope、装配位置、触发器 | D2（需 Research Gate + Architecture Gate） |
| Persona Markdown / Frozen Core 投影变化对 prefix 的影响 | **R3 已交付盘点**（今天注入紧凑 JSON；D1 改载体） |
| 用户机器真实 `.laputa` / `sessions/` 体积 | R4 |
| BML `Identity` 等 kind 与 Persona 双权威 | D0 |
| `run_startup_gc` 是否接线、Reset/Delete 是否调 `on_session_end` | D2 / session 生命周期 |
| 活体跨 session / 多 channel smoke | Architecture Gate 后的桌面验收 |
| Garden 独立仓库是否将来做 STM UI | 超出本 EPIC；入口已冻结在 Diva Memory 页 |
