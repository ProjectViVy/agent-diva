# 频道能力参考实现对照：迭代摘要

## 状态

Research / Proposal；待正式立项，未授权生产实现。

## 本次产出

- 新增 ZeroClaw、Octos、OpenFang 与 agent-diva 的频道能力对照研究包。
- 明确“Octos 作为主结构对照、ZeroClaw 作为能力/可靠性标杆、OpenFang 作为 Bridge/TCK
  参考”的组合结论。
- 分别核对 QQ、DingTalk、Feishu 的 transport、消息合同、媒体、线程、重连和测试差异。
- 在 `TODOLIST.md` 新增 `CHANNEL-CAPABILITY-CONTRACT-EPIC`，登记统一 envelope、能力矩阵、
  TCK、pacing/backpressure 和 supervisor/reconnect 的待立项工作。
- 未修改任何生产 Rust 代码。

## 关键决策建议

近期以 Octos 的结构降低 agent-diva 改造成本；QQ 能力按 ZeroClaw 的上限补齐；DingTalk
保留 agent-diva 现有 Stream/媒体路径；Feishu 吸收 Octos 的 reply/edit/delete/mock 测试，
再按 ZeroClaw 补 draft、reaction 和 approval。
