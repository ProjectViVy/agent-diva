# P2-10: 热路径 clone + 同步 IO

## 问题描述

pro 分支的 agent loop 相比 main 扩展了更多运行时能力，包括文件附件、运行时控制、prefetch recall、soul governance、Mentle/custom tools、子代理 spawner、cron 保护和事件流。这些能力增加了热路径上的 clone 和同步 IO 暴露面。

主入口位于 `agent-diva-agent/src/agent_loop/loop_turn.rs::process_inbound_message_inner`。当前每个 turn 会构造完整消息上下文，然后在最多 `max_iterations` 次循环中调用 provider：

```rust
// agent-diva-agent/src/agent_loop/loop_turn.rs:152-169
let tool_defs = if msg.channel == "cron" || is_cron_trigger {
    self.tools.get_definitions().into_iter().filter(...).collect()
} else {
    self.tools.get_definitions()
};
let mut stream = self
    .provider
    .chat_stream(
        messages.clone(),
        ...
        Some(model_to_use.clone()),
        4096,
        0.7,
    )
    .await?;
```

这里的 `messages.clone()` 是最高影响的 clone 点。`messages` 包含系统 prompt、历史消息、prefetch 注入、工具调用和工具结果，且会随着每轮工具调用增长。

事件广播路径也有高频 clone：

```rust
// agent-diva-agent/src/agent_loop/loop_turn.rs:138-147
let event = AgentEvent::IterationStarted { ... };
if let Some(tx) = event_tx {
    let _ = tx.send(event.clone());
}
let _ = self.bus.publish_event(msg.channel.clone(), msg.chat_id.clone(), event);
```

流式输出、reasoning delta、tool delta、tool start、tool finished 和 final response 都遵循类似模式。尤其是工具结果完成事件会先 clone 完整结果：

```rust
// agent-diva-agent/src/agent_loop/loop_turn.rs:362-373
let event = AgentEvent::ToolCallFinished {
    name: tool_call.name.clone(),
    is_error: result.starts_with("Error"),
    result: result.clone(),
    call_id: tool_call.id.clone(),
};
```

该 clone 发生在 `self.context.add_tool_result(...)` 之前，若工具返回大文本，事件通道仍会复制完整结果。

`agent-diva-agent/src/agent_loop.rs` 的构造路径也有大量 config/Arc/path clone：

```rust
// agent-diva-agent/src/agent_loop.rs:204-210
ToolAssembly::new(workspace)
    .builtin(tool_config.builtin.clone())
    .with_network_config(tool_config.network.clone())
    .mcp_servers(tool_config.mcp_servers.clone())
```

```rust
// agent-diva-agent/src/agent_loop.rs:442-448
let tools = build_agent_tools(
    workspace.clone(),
    &tool_config,
    spawner.clone(),
    file_manager.clone(),
    custom_tools.clone(),
    tool_config.cron_service.clone(),
);
```

这些构造期 clone 单次成本可接受，但 pro 分支新增的 `custom_tools`、`mcp_servers`、`cron_service`、`file_manager` 和 memory provider 组合让对象图更大，后续刷新工具集时成本更明显。

同步 IO 主要集中在 `agent-diva-core/src/memory/manager.rs`。该类型实现了 async `MemoryProvider`，但内部仍使用 `std::fs`：

```rust
// agent-diva-core/src/memory/manager.rs:48
std::fs::read_to_string(&self.memory_path)

// agent-diva-core/src/memory/manager.rs:60-62
std::fs::create_dir_all(parent)?;
std::fs::write(&self.memory_path, &memory.content)?;

// agent-diva-core/src/memory/manager.rs:83-89
let mut content = self.load_history();
...
std::fs::write(&self.history_path, content)?;
```

这些方法会被 async trait 方法调用：

```rust
// agent-diva-core/src/memory/manager.rs:262-281
async fn sync_turn(&self, request: SyncTurnRequest) -> crate::Result<SyncTurnResponse> {
    ...
    self.save_memory(&memory)
    ...
    self.append_history(history_entry)
}
```

因此正常对话后的 memory sync 可能在 Tokio worker 上执行同步文件读写。`append_history` 还是全量读 `HISTORY.md` 后重写全文件，文件越大成本越高，并且缺少 per-file 写锁或追加式写入保护。

## 与 main 分支的差异说明

根据 `.hermes/audit/branch-ownership.md`，P2-10 归属为 both：main 和 pro 都存在 clone 与同步 IO 问题，但 pro 的 agent loop 差异极大，需要在 pro 内单独分析和修复。

pro 相比 main 增加或放大的 clone 热点包括：

1. `loop_turn.rs` 增加 prefetch recall，构造 `PrefetchRequest` 时 clone `workspace`、`channel`、`prefetch_user_message`。
2. 事件链路更丰富，流式 delta、reasoning delta、tool delta、tool start/finish、final response 都会为了同时发给 `event_tx` 和 `MessageBus` 进行事件和 channel/chat id clone。
3. cron trigger 保护每轮重新生成并过滤 `tool_defs`，`self.tools.get_definitions()` 在循环内重复执行。
4. tool result 在广播事件时先 clone 完整结果，再追加到 LLM 上下文。
5. `agent_loop.rs` 构造路径加入 `SubagentManager`、`FileManager`、Mentle runtime、custom tools、memory provider 和 cron service，导致配置对象 clone 更多。

loop_turn.rs 的 clone 模式可以分为三类：

1. 必要所有权转移：provider API 当前接收 `Vec<Message>`，因此调用处使用 `messages.clone()`。
2. 多消费者广播：同一个 `AgentEvent` 同时发给 UI `event_tx` 和 bus，因此 `event.clone()`、`msg.channel.clone()`、`msg.chat_id.clone()` 高频出现。
3. 可优化预览/结果复制：`args_str.clone()`、`preview.clone()`、`result.clone()`、`final_content.clone()` 中部分可以通过先截断、引用或改变事件载荷来降低成本。

memory/manager.rs 的同步 IO 问题两边都有，但 pro 的影响更明显，因为 pro 的 `AgentLoop` 默认把 `MemoryManager` 作为 `Arc<dyn MemoryProvider>` 接入，并在 turn 中执行 prefetch、consolidation、sync_turn 等 memory provider 边界。同步文件 IO 更容易出现在正常异步对话流程中。

## 影响评估

优先级为 P2。该问题不是直接安全漏洞，但会造成性能瓶颈、延迟抖动和长会话退化。

主要影响：

1. 每轮 `messages.clone()` 会随上下文增长线性放大。工具调用越多、历史越长，复制成本越高。
2. `get_definitions()` 每轮重建工具 schema，若 MCP/custom tools 较多，会增加分配和 JSON 构造成本。
3. 大工具结果会在事件广播前完整 clone，可能造成瞬时内存峰值和 UI/bus 背压。
4. `MemoryManager::append_history` 全量读写 `HISTORY.md`，长期运行后每次 append 成本随文件大小增长。
5. `std::fs` 在 async-facing 路径中运行，可能阻塞 Tokio worker，导致同 runtime 上的 provider stream、工具调用或 channel 处理延迟。

## 解决方案

建议按收益和风险分阶段处理。

第一阶段，降低最明显的热路径复制：

1. 缓存工具定义。给 `ToolRegistry` 增加 revision 或 dirty 标记，`loop_turn.rs` 每个 turn 只构建一次普通工具定义和一次 cron-filtered 工具定义，循环内复用。
2. 工具结果事件先截断再广播。例如 `AgentEvent::ToolCallFinished` 只携带 preview、长度和错误标记，完整结果只进入 LLM 上下文或持久化层。
3. 对 `messages.clone()` 做 API 层评估。可选方案包括 provider 接口接受 `&[Message]`、`Arc<[Message]>`，或在 provider 边界内部序列化时按引用读取，避免调用方复制完整 Vec。
4. 对 `model_to_use.clone()`、`msg.channel.clone()`、`msg.chat_id.clone()` 等小对象不必优先处理，除非 profiling 证明高频事件广播成为瓶颈。

第二阶段，整理 pro 构造路径的大对象 clone：

1. 将稳定运行期配置改为共享不可变 `Arc`，例如大的 MCP server map、network config、custom tools 列表。
2. 工具刷新时只 clone 必要 Arc，不复制整个配置对象。
3. 子代理和主 agent 共享只读工具配置快照时，使用明确的 config snapshot 类型，避免散落 clone。

第三阶段，迁移 MemoryManager IO：

1. 将 async-facing 的 memory provider 方法改为使用 `tokio::fs`，或者把现有同步方法包入 `tokio::task::spawn_blocking`。
2. `append_history` 改为追加写，而不是 `load_history + write whole file`。如果仍需 Markdown 空行格式，可用 append 模式写入 `entry.trim_end()` 和分隔换行。
3. 为 `MEMORY.md` 和 `HISTORY.md` 写入增加 per-file async mutex，避免并发 `sync_turn` 丢写。
4. 后续与 P0-3 原子持久化修复合并考虑：写 `MEMORY.md` 这类替换式文件时使用 `tempfile + fsync + atomic rename`。
5. 对 `load_memory` 增加大小上限或缓存策略，避免每次 prompt 构建读取过大的 `MEMORY.md`。

## 验证方法

建议验证命令：

```powershell
cargo test -p agent-diva-agent
cargo test -p agent-diva-core memory
cargo clippy -p agent-diva-agent -- -D warnings
cargo clippy -p agent-diva-core -- -D warnings
```

建议新增性能/回归测试：

1. 构造含 50 条历史、多轮工具调用、大工具结果的 turn，记录 provider 调用前后 allocations 或 wall time，确认 `messages.clone()` 优化后没有行为回归。
2. 验证 `ToolCallFinished` 事件不会携带超过配置上限的 result preview，但 LLM 上下文仍收到按既有策略处理后的工具结果。
3. 构造大 `HISTORY.md`，执行多次 `sync_turn`，确认不再全量读写历史文件。
4. 并发调用 `sync_turn`，确认 `HISTORY.md` 不丢条目、不交错破坏 Markdown 结构。
5. 使用 `rg "std::fs::" agent-diva-core/src/memory/manager.rs` 确认 async-facing 路径已迁移或被明确隔离在 blocking 线程。

人工 smoke 验证可使用长会话和大工具输出运行一次真实 CLI 对话，观察响应流是否持续输出、最终 session/memory 文件是否正确更新。

## 优先级

P2。该问题主要影响高负载、长会话和批量工具/子代理场景，应在 P0/P1 稳定性问题之后处理，但建议与 memory/session 原子写入改造一起规划，避免重复改动持久化层。
