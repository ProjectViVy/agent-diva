# v0.0.1 — 验证记录

## 验证方式

| 项目 | 方法 | 结果 |
| --- | --- | --- |
| Claude Code 模式数量 | 多源交叉验证（官方文档 + 4 个教程站） | 6 种模式：plan/dontAsk/default/acceptEdits/auto/bypassPermissions |
| agent-diva 现状 | 源码逐点核实（policy.rs / exec_policy.rs / guardian.rs / orchestrator.rs / ChatView.vue / App.vue / handlers.rs） | 行号引用全部与实码一致 |
| 差距结论 G1-G4 | 代码路径比对（guardian 分支、exec_policy 分支、orchestrator 生产路径） | 证据成立 |

## 局限性

- Claude Code 官方文档无法直连（网络受限），模式说明以可访问的多源教程为准；
  若需逐字核对官方措辞，可在网络恢复后复核 code.claude.com/docs/zh-CN/permission-modes。
- 未执行任何代码变更，无测试/构建验证项。
