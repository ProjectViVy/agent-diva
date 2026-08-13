# E0 运行时表征与诚实产品状态

## 结果

- 普通 `POST /api/autodream/runs` 手动触发现在会执行受限 AutoDream worker，并返回
  completed、failed 或 cancelled 终态，不再留下无法推进的 running 记录。
- 运行记录增加稳定、无 Memory 内容的 `failure_code`，覆盖输入不可用、超时、取消、
  worker 失败、报告生成失败与陈旧运行恢复。
- Evolution 页面增加建设中告警，明确当前候选仍由旧规则生成，不能把底层提案能力
  表述成已经可用的自动进化闭环。
- 既有 Laputa proposal 边界保持不变：worker 只能提交候选，不能直接修改 typed
  Memory，也不能自行批准。

## 影响范围

`agent-diva-core` 的 AutoDream 运行 DTO、`agent-diva-autodream` 运行状态写入、
Manager 手动触发入口，以及 Evolution GUI 的状态呈现。

## 非目标

E0 不实现 Experience Journal、LLM reflection、Candidate Gate、持久队列或最终桌面
验收。这些依次属于 E1–E7 与 G2D+。
