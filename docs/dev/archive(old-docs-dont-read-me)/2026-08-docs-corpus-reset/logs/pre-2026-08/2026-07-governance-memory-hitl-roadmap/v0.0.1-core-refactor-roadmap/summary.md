# Governance × Memory × Human-in-the-loop 重构排期总结

本次仅完成规划，不修改运行时代码。

在根 `TODOLIST.md` 增加 10 周、6 个阶段的核心重构路线，覆盖现状刻画、统一治理领域模型、Memory Framework 2.0、Human-in-the-loop 闭环、Agent Loop 接入、灰度迁移与收口。计划以当前 `agent-diva-pro` 的真实 crate 和产品边界为基线，明确禁止引入第二套 runtime、审批存储或 Memory 权威。

计划包含活动编号 `GMH-00` 至 `GMH-53`、阶段 Gate、里程碑、依赖关系、并行限制、人员假设以及统一完成定义。

## 2026-07-30 架构修订

原排期把 Mentle/索引作为长期检索层的假设已经废止。治理分支已经完成
Embedded Laputa 和 Mentle clean-break 的设计与验证，因此当前计划改为：

- 保留 GMH-21/22 合同和 GMH-23 阶段 1/2 proposal 边界；
- 暂停 GMH-23 阶段 3 与旧 GMH-24；
- 新增 GMH-23A..23D，依次冻结架构、移植 typed SQLite+FTS5 store、
  接入 recall、恢复 apply/HITL；
- GMH-24 改为 Embedded Laputa cutover 与 Mentle/LLVM 删除 Gate；
- 不合并治理分支的整体 runtime rewrite，只定向回迁文档、合同、测试思想
  和适配当前 `agent-diva-pro` 所需的最小实现。

根 `TODOLIST.md` 是修订后的执行真相。
