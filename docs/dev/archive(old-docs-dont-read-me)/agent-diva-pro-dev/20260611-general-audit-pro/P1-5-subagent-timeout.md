# P1-5: 子代理无 timeout/并发上限

## 问题描述

pro 分支的 `agent-diva-agent/src/subagent.rs` 已经重写了子代理实现，当前同时存在两类执行路径：后台子代理 `SubagentManager::spawn` 和批量隔离子代理 `SubagentManager::spawn_batch`。

后台子代理路径仍然是 fire-and-forget：

```rust
// agent-diva-agent/src/subagent.rs:202
let bg_task = tokio::spawn(async move {
    Self::run_subagent(...).await;

    let mut tasks = running_tasks.lock().await;
    tasks.remove(&task_id_clone);
});

// agent-diva-agent/src/subagent.rs:227
let mut tasks = self.running_tasks.lock().await;
tasks.insert(task_id.clone(), bg_task);
```

该路径会把 `JoinHandle<()>` 存入 `running_tasks`，但没有 `tokio::time::timeout` 包裹 `run_subagent`，也没有取消入口、abort API 或任务完成状态回收。`exec_timeout` 会继续传入工具构造：

```rust
// agent-diva-agent/src/subagent.rs:503-507
let tools: ToolRegistry = ToolAssembly::new(workspace.to_path_buf())
    .builtin(builtin_tools.clone())
    .with_network_config(network_config.clone())
    .with_exec_timeout(exec_timeout)
```

但这只约束工具执行配置，不约束整个子代理 LLM 循环。`execute_subagent_task` 内部仍可能最多 15 轮调用 provider 和工具：

```rust
// agent-diva-agent/src/subagent.rs:523-534
while iteration < max_iterations {
    iteration += 1;

    let response = provider
        .chat(
            messages.clone(),
            Some(tools.get_definitions()),
            Some(model.to_string()),
            2000,
            0.7,
        )
        .await?;
```

批量隔离路径已经有 timeout 逻辑：

```rust
// agent-diva-agent/src/subagent.rs:336-345
let exec_result = tokio::time::timeout(
    timeout_duration,
    Self::execute_isolated_task(...),
)
.await;
```

但是 `spawn_batch` 对每个输入任务直接 `join_set.spawn`，没有 `Semaphore`、`buffer_unordered` 或显式 `MAX_CONCURRENT`：

```rust
// agent-diva-agent/src/subagent.rs:259-269
let mut join_set = JoinSet::new();

for task in request.tasks {
    ...
    join_set.spawn(async move {
        Self::run_isolated_subagent(...).await
    });
}
```

因此，批量任务数量较大时仍会一次性启动大量 provider/tool 调用。

## 与 main 分支的差异说明

根据 `.hermes/audit/branch-ownership.md`，P1-5 归属为 both：main 和 pro 都需要修复，但 pro 已大幅改写 `subagent.rs`，不能直接套用 main 的修复。

pro 相比 main 的主要改进：

1. 增加了 `exec_timeout` 配置，默认值为 30 秒，见 `SubagentManager::new` 的 `exec_timeout.unwrap_or(30)`。
2. 增加了 `spawn_batch` 批量隔离子代理路径，并使用 `JoinSet` 收集任务结果。
3. `run_isolated_subagent` 已经用 `tokio::time::timeout` 包裹 `execute_isolated_task`，超时会返回 `SubAgentStatus::Timeout`。
4. `spawn_batch` 能把 panic 转换为 `SubAgentStatus::Error`，比单纯丢弃 `JoinHandle` 更可观测。

仍存在的问题：

1. 后台 `spawn` 路径没有整体 timeout。`exec_timeout` 只传给工具层，不能限制 provider 调用、循环总时长或结果公告流程。
2. 后台 `spawn` 没有取消 API。`running_tasks` 保存了 `JoinHandle<()>`，但没有 `cancel_subagent(task_id)` 或 shutdown 时统一 abort。
3. `spawn_batch` 没有并发上限。`JoinSet` 只负责收集结果，不提供背压。
4. `running_tasks` 存在插入与完成清理的竞态窗口：后台任务可能很快完成并执行 `remove`，而外层随后才把 handle 插入 map，导致已完成 handle 残留。
5. `parent_tool_limits` 字段当前没有参与并发控制，未形成可配置的子代理资源预算。

## 影响评估

优先级为 P1。该问题会影响生产可控性和资源稳定性。

长任务或卡住的 provider 请求可能使后台子代理长期占用 runtime、provider 配额和工具资源。批量任务未限流时，单次请求可以并行启动大量 LLM 调用和工具调用，压垮本地工具、MCP server、网络 provider 或文件系统。

可观测性也不足。后台 `spawn` 的 `JoinHandle<()>` 不被 await，panic、超时和取消结果无法以结构化状态暴露给调用方；`get_running_count` 只能看到 map 当前数量，不能反映超时、失败、已取消等生命周期状态。

## 解决方案

针对 pro 分支建议分两层修复。

第一层，补齐后台 `spawn` 生命周期控制：

1. 在 `SubagentManager` 中增加并发控制，例如 `Arc<tokio::sync::Semaphore>`，并从配置或 `ToolLimits` 派生 `max_concurrent_subagents`。
2. 在 `spawn` 开始时先注册任务占位，再启动 `tokio::spawn`，避免完成清理早于 map 插入的竞态。
3. 对 `run_subagent` 包裹 `tokio::time::timeout(Duration::from_secs(exec_timeout), ...)`，超时后公告明确的 timeout 结果，并清理 registry。
4. 增加 `cancel_subagent(task_id)`，从 `running_tasks` 取出 handle 后 `abort`，并可选择 await 一个短 timeout 确认退出。
5. 在 agent shutdown 或 runtime control 停止路径中统一取消仍在运行的子代理。

第二层，给 `spawn_batch` 加背压：

1. 引入 `max_concurrent_batch_subagents`，默认使用保守值，例如 4 或 CPU/配置相关值。
2. 不要一次性把所有 task 放入 `JoinSet`；改为先填充到并发上限，`join_next` 回收一个后再补一个。
3. 或者使用 `futures::stream::iter(tasks).map(...).buffer_unordered(limit)`，并保留当前 `SubAgentResult` 错误转换语义。
4. timeout 保留在单任务 `run_isolated_subagent` 内，同时可增加整个 batch 的可选总 timeout，避免超大 batch 长时间占用调用方。

建议新增结构化任务状态：

```rust
struct RunningSubagent {
    handle: JoinHandle<()>,
    started_at: Instant,
    label: String,
    origin_channel: String,
    origin_chat_id: String,
}
```

这样 `get_running_count` 可以扩展为 `list_running_subagents`，便于 GUI/CLI 展示和取消。

## 验证方法

建议增加以下测试和验证：

```powershell
cargo test -p agent-diva-agent subagent
cargo test -p agent-diva-agent timeout_status_is_distinct
```

新增测试建议：

1. 后台 `spawn` 执行超过 `exec_timeout` 时会清理 `running_tasks`，并发布 timeout 公告。
2. `cancel_subagent(task_id)` 会 abort 对应任务，重复取消返回明确状态。
3. `spawn_batch` 在 10 个以上任务输入时，最大同时运行数不超过配置值。
4. 子代理快速完成时，`running_tasks` 不残留已完成 handle。
5. panic、timeout、cancel 三种状态都有可观测结果，不会只在 Tokio runtime 日志中体现。

人工 smoke 验证可通过构造一个包含 sleep/阻塞模拟工具的子代理任务，确认超时后 `get_running_count()` 回到 0，后续子代理仍能正常启动。

## 优先级

P1。pro 已经补了一部分 batch timeout，但后台子代理和批量并发上限仍缺失，建议作为 pro 分支独立修复项处理。
