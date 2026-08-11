# C5d Bounded Context Recovery Closure

C5d 将 context assembly 的顶层预算报告收敛为三个 region：`StablePrefix`、
`CanonicalCheckpoint` 和 `ActiveTail`。稳定 system/core schema、单一 checkpoint、历史/工具
结果/当前回合分别落入明确 region；报告同时保留细粒度 layer 计量，便于诊断而不再制造
第二套上下文状态。

完整 canonical checkpoint 参与总预算计算。自动、manual 和 reactive 路径继续使用
`CheckpointCompactor::compact_snapshot`；机械工具组折叠和 artifact 化先于 provider semantic
compact。compaction/provider/quality 失败不写入 durable checkpoint，也不再回退到旧的固定
消息数截断，保留原始 active transcript 供下一次恢复。

实现提交：`e25a97fd`。
