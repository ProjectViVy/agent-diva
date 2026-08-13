# COGNITIVE-R0：当前系统、数据与耦合全景盘点

- 状态：`Research Complete (package) / Research Gate Pending User Review`
- 记录日期：2026-08-13
- EPIC：`LAPUTA-COGNITIVE-WORKSPACE-RESET` → **R0**
- 性质：当前实现事实盘点；**不授权**目标架构定稿或代码实施
- 配套：R1 Evolution 切片已先行交付，本包引用、不重写

## 一句话结论

当前运行时**不是**四个独立工作区，而是一条共享的 `EvolutionProposal → MemoryGovernanceCoordinator → Approval Center` 混域链，旁边再挂 Skill 文件树、WORLD 独立账本、危险工具审批和 C1–C5 会话上下文。

产品已冻结的 Persona / Memory / Evolution / Approval 边界，与这份实现地图不一致。R2–R4 必须以本包的路径、符号和失败基线为输入，而不是沿旧链打补丁。

## 阅读顺序

1. [current-state-map.md](./current-state-map.md) — 系统、读写、事件、Prompt、GUI 全景
2. [dependency-and-data-inventory.md](./dependency-and-data-inventory.md) — 持久化、符号、写入者、KEEP/RENAME/DELETE/DECIDE
3. [legacy-failure-baseline.md](./legacy-failure-baseline.md) — 三条桌面症状的触发链与验收证伪条件
4. 已有切片：[R1 Evolution 切片](../cognitive-r1-genericagent-evolution-2026-08/r0-evolution-slice.md)

## EPIC 完成物映射

| EPIC 要求 | 本包文件 |
| --- | --- |
| `current-state-map.md` | 同名 |
| `dependency-and-data-inventory.md` | 同名 |
| `legacy-failure-baseline.md` | 同名 |

## 证据分级

| 标签 | 含义 |
| --- | --- |
| 源码事实 | 当前树可定位实现 |
| 提交事实 | git / 已落地修补记录 |
| 实验观察 | 本轮静态核对；无新桌面复测 |
| 推断 | 由事实推导 |
| 建议 | 研究标记，非架构批准 |

## 产品约束（研究不得推翻）

- Memory CRUD 不走审批；BML 是普通长期 Memory 唯一权威
- `memory_md` / `MemoryMd` 属于 clean-break 删除范围
- Persona 是四份 Markdown + WORLD，不等于 Memory
- Evolution 只管理 Skill；旧 AutoDream–Evolution–Governance 不再修补
- Chat Approval Center 只保留危险运行时授权
- 本轮采用 clean break；不设计导入、双读写或 runtime fallback

## 研究 Gate 自检

- [x] 三份 EPIC 完成物齐全
- [x] 引用源码路径与符号可复现
- [x] 结论区分源码事实 / 提交事实 / 推断 / 建议
- [x] 三症状固化为失败基线，不先打补丁
- [x] KEEP / RENAME / DELETE / DECIDE 矩阵可供 R4 引用
- [x] R1 Evolution 切片被引用，不重复成第二份权威
- [x] 未解决问题显式列出
- [ ] 用户 Research Gate 评审（待）

## 明确不做

目标 DTO/schema、STM 物理权威、Persona revision store、SOP/Skill 晋升状态机、删除切片、保护性分支创建、生产代码修改。

## 开放缺口（交给后续研究）

| 缺口 | 交给 |
| --- | --- |
| STM 存储 / 并发 / 装配 / 失败矩阵 | R2 |
| Persona Markdown revision / Diff / 编辑器选型 | R3 |
| WORLD 投影是否进入 Prompt、WorldGovernance 写入者 | R3 |
| 用户 profile 数据抽样与删除影响 | R4 |
| AutoDream 报告 / Notebook 是否独立于 Evolution | R4 / D3 |
| BML `MemoryRecordKind::{Identity,…}` 与 Persona 权威关系 | D0 |
