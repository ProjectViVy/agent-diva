# C1c Workspace Memory Epoch Summary

## 完成内容

- 复用 BML `store_revision` 作为工作区级 authority epoch，无新增数据库 schema。
- `MemoryProvider` 新增启动投影刷新契约；Typed Provider 维护 authority revision 与
  startup projection revision，并以 single-flight 合并重复/乱序刷新。
- Manager 的 Typed apply、恢复、幂等重放和 rollback 成功路径接入 Runtime Control；
  Runtime 只刷新共享 Provider，Session 在下一轮上下文组装时按已有 revision 选择性重建
  `MemoryPolicyAndIndex`。
- 新增 workspace ID 校验和 runtime 队列控制优先级；通知不是用户消息，不写入 Session
  历史，不触发额外 LLM 调用。

## 未包含

- Skills upload/delete 与 `memory_distill` 的显式 reload 通知仍是独立待办。
