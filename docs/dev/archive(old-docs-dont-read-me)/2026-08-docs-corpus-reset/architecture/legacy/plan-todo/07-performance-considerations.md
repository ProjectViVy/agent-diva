# 性能考量

计划与 TODO 均为单会话小集合，策略判断必须是纯内存 O(1) 查表；不要在每个工具调用时扫描完整工作区或重新序列化完整会话。当前 `TodoList` 以 revision 管理，[model.rs](agent-diva-core/src/planning/model.rs:177)，适合以 compare-and-swap 承载并发保护。

- `snapshot_active_plan_runtime` 当前要读取 plan、steps、todos，[loop_runtime_control.rs](agent-diva-agent/src/agent_loop/loop_runtime_control.rs:365)；工具循环中应每轮缓存一次 snapshot，状态变更后失效。
- plan 内容/步骤详情只在 GUI 打开卡片时加载；会话列表仅传摘要、状态、revision、进度计数。
- TODO patch 传递变更项而非完整列表；保留全量写兼容端点但限制为迁移期。
- 以 500 步、2,000 TODO 的合成用例量测策略、序列化、GUI 投影；目标是不让单次状态投影阻塞 agent 流。
