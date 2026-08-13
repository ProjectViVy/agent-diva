# 发布

本切片不发布、不 push。命令为显式离线操作，默认没有隐含 apply；运行时 AgentLoop
不会自动扫描旧 session。

rollback 使用 apply manifest，重复 rollback 明确失败。删除/损坏 manifest 时不猜测
目标，也不进行宽泛 Journal 清理。
