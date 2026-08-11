# C5b Canonical Checkpoint Summary

C5b 将 session 的追加式压缩状态收敛为最多一个有界的
`canonical_checkpoint_v1`。检查点保存版本、ID、触发类型、覆盖的 durable message
index、源消息/token 统计、质量信息和固定七段正文，正文上限为 8,000 字符；原始
transcript 继续完整保存在 session 中。

自动、手动和 provider overflow reactive 压缩现在都进入
`CheckpointCompactor::compact_snapshot`。输入由旧检查点、完整 durable transcript 和
可选当前 turn 组成；成功后 replacement checkpoint 原子替换旧状态。reactive 更新在
turn-local `PendingCheckpointUpdate` 中等待，只有 finalize 保存当前 turn 后才提交。

裁剪边界按完整 turn/tool group 选择。完成的 assistant tool-call 与匹配结果被机械折叠为
工具名、完成状态和 artifact 引用；未完成、缺结果或当前执行中的组留在 active tail。
ContextBuilder 固定按 stable prefix → 单一 checkpoint → active history → dynamic
sections → current user 渲染，不再注入多份摘要或多重 boundary marker。

实现提交：`19a5bfec`；边界测试提交：`c6e3ace4`。
