# E4 提案治理与 rejection suppression

本切片把 E3 候选完整接入 Laputa 的通用 Governance Ledger 决策路径，并补齐拒绝后的
重复提案抑制：

- accepted candidate 一对一发布为 PendingReview proposal，不改变 authority。
- proposal 编辑后按新 digest 创建治理 request，并撤销旧 pending/allowed request；
  旧 receipt 无法继续 apply。
- 决策已写 Governance Ledger、但 proposal 状态尚未写入的崩溃窗口可通过同一决策
  请求补齐；重复请求返回既有结果，不重复决定。
- deny 后记录 content-only digest、proposal ID、拒绝时间和 90 天有效期；文件最多
  保留 1000 条且不含候选内容。
- suppression 文件丢写时可从 rejected proposal 重建；损坏文件 fail closed。
- 下一次 AutoDream 对同内容给出 `suppressed` gate reason；内容显著变化允许重提。
- Manager 将 version、idempotency、consumed、expired、invalid transition 分别映射为
  稳定 reason code。

本切片不自动批准、不自动 apply，也不改变 typed Memory authority。
