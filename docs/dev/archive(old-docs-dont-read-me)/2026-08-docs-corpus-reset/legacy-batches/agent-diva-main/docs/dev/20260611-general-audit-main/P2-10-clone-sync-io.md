# P2-10: 热路径 clone + 同步 IO

## 问题描述

P2-10 被归类为 both：main 和 pro 都存在性能/资源问题，但热路径形态不同。main 分支当前主循环入口在 `agent-diva-agent/src/agent_loop.rs`，真正的单轮处理逻辑已拆到 `agent-diva-agent/src/agent_loop/loop_turn.rs`。

### 1. agent loop 中的 clone 使用模式

`agent-diva-agent/src/agent_loop.rs` 前 500 行主要是构造、依赖注入和入口调度，clone 以共享运行时对象和配置复制为主：

- `agent-diva-agent/src/agent_loop.rs:173-182` 在 `AgentLoop::new` 中 clone `workspace`、`provider`、`bus`、`model` 传入 context、session、subagent manager。
- `agent-diva-agent/src/agent_loop.rs:276-304` 在 `with_tools_and_memory_provider` 中 clone `workspace`、`provider`、`bus`、`tool_config` 下的 builtin/network/mcp/subagent/context_budget、`file_manager`，用于构造 `SubagentManager` 和 `ToolAssembly`。
- `agent-diva-agent/src/agent_loop.rs:323-334` 将 `tool_config.clone()`、trace/debug logger clone 保存到 `AgentLoop`。
- `agent-diva-agent/src/agent_loop.rs:337-349` cron service 存在时再次用 cloned config 重建工具 registry。
- `agent-diva-agent/src/agent_loop.rs:484-497` 每条 inbound message 会 clone 一份 `msg` 作为错误上下文，后续又 clone channel/chat/sender metadata。

这些 clone 多数发生在构造期或错误路径，优先级低于每轮 LLM/tool 路径。

主热路径在 `agent-diva-agent/src/agent_loop/loop_turn.rs`：

- `loop_turn.rs:51` 对 `msg.content` 做 preview clone。
- `loop_turn.rs:134` `user_attachments.clone()` 用于把附件引用写入 session。
- `loop_turn.rs:151-156` prefetch request clone `workspace`、`channel`、`prefetch_user_message`。
- `loop_turn.rs:208-212`、`455-491`、`626-630`、`741-745`、`893-897` 高频事件发布路径会 clone event、channel、chat_id。
- `loop_turn.rs:219-229` 每次 agent iteration 都调用 `self.tools.get_definitions()`；cron 场景还要过滤工具定义。
- `loop_turn.rs:310-318` provider streaming 调用移动 `provider_messages`，但仍 clone `tool_defs` 和 `model_to_use`。
- `loop_turn.rs:579-581` 工具调用分支会 clone response content、tool_calls、reasoning_content 追加到上下文。
- `loop_turn.rs:616-623` 工具参数先序列化为 string，再 clone preview/name/call_id 发事件。
- `loop_turn.rs:700` raw debug payload clone `params_value`。
- `loop_turn.rs:735-738` `AgentEvent::ToolCallFinished` clone 完整 `result`。如果工具输出很大，事件总线会复制完整结果，然后 LLM context 才可能做截断。
- `loop_turn.rs:760` 在开启 tool output summaries 时再次 clone 完整 result 到 trace metadata。
- `loop_turn.rs:826-827` tool result 写回 context 时 clone call id/name。
- `loop_turn.rs:886-897` final response clone 用于 preview、event 和 outbound。

当前 main 分支已经通过 `prepare_budgeted_messages(&messages, ...)` 避免了旧报告中“每次直接 `messages.clone()`”的显式模式，provider 调用也移动 `prepared_request.messages`。但仍有大量事件、debug、tool result 和 config clone，尤其是完整工具输出在广播/trace 前未截断。

### 2. memory/manager.rs 中的同步 std::fs 写入

`agent-diva-core/src/memory/manager.rs` 仍是同步文件 IO：

- `manager.rs:48` `load_memory` 使用 `std::fs::read_to_string`。
- `manager.rs:60-62` `save_memory` 使用 `std::fs::create_dir_all` 和 `std::fs::write`。
- `manager.rs:69` `load_history` 使用同步 read。
- `manager.rs:81-89` `append_history` 先同步读完整 `HISTORY.md`，追加后同步整文件写回。
- `manager.rs:99` `load_daily_note` 使用同步 read。
- `manager.rs:120-122` `save_daily_note` 使用同步 create/write。
- `manager.rs:130`、`manager.rs:191` `list_notes` / `list_memory_files` 使用同步 `read_dir`。

这些同步函数又被 async trait 路径调用：

- `manager.rs:262` `MemoryProvider::sync_turn` 是 async，但内部直接调用 `save_memory` 和 `append_history`。
- `agent-diva-agent/src/agent_loop/loop_turn.rs:915` 附近在 turn 后调用 memory consolidation，最终会进入 memory provider 写入路径。
- `agent-diva-agent/src/agent_loop.rs:464-466` shutdown 时调用 `memory_provider.on_session_end(...)`，当前默认实现不写文件，但接口语义是 async。

风险不是 Rust 类型层面的数据竞争，而是 async runtime worker 被同步磁盘 IO 阻塞，并且 `append_history` 是读全量、改内存、写全量；并发 sync_turn 时可能发生丢写。

### 3. 可以改为引用或 Arc 的 clone

优先处理以下 clone：

- `ToolConfig` 中较大的不可变配置：`builtin`、`network`、`mcp_servers`、`subagent_policy` 可在 `AgentLoop`/`SubagentManager` 之间使用 `Arc` 或内部 `Arc<RwLock<...>>` 共享，减少构造和刷新时复制。
- `mcp_servers: HashMap<String, MCPServerConfig>`：`agent_loop.rs` 和 `subagent.rs` 均会 clone，MCP server 配置数量增多时成本上升。可改成 `Arc<HashMap<...>>` 或配置快照版本号。
- `tool_defs`：每次 iteration 都重建和 clone。应由 `ToolRegistry` 提供“定义缓存 + revision”或 `Arc<[serde_json::Value]>`，只在工具集合变化时重算。
- `AgentEvent::ToolCallFinished.result`：不要 clone 完整结果进入事件；改为 preview、byte_count、is_truncated，或把完整结果放在 debug raw 且受开关控制。
- `msg.channel` / `msg.chat_id`：事件发布接口可以接受 `&str` 或内部只 clone 一次到局部 `Arc<str>`，避免高频流式 delta 每次复制。
- `final_content`：最终响应 clone 一次通常可接受，但 preview 可以用借用生成，事件和 outbound 可通过调整顺序减少一次 clone。

不建议优先处理的 clone：

- `Arc::clone(provider/bus/file_manager/logger)` 成本很低。
- 构造期 `workspace.clone()`、`model.clone()` 对正常请求延迟影响有限。
- 错误路径 metadata clone 不属于首要性能瓶颈。

### 4. 需要改为 tokio::fs 或 spawn_blocking 的同步 IO

优先迁移：

- `MemoryManager::save_memory`、`append_history`、`save_daily_note`：写路径会发生在 agent turn 后，应改为 async 写入。
- `MemoryManager::load_memory`：prompt/system block 构建会读取 MEMORY.md，应避免在 async 入口同步读。
- `MemoryManager::append_history`：不能继续读全量再写全量；应改为 append-only 或带锁的原子写。
- `list_notes` / `list_memory_files`：如果由 GUI 或工具频繁调用，改为 `tokio::fs::read_dir`；若保留同步 API，则在 async 调用点用 `spawn_blocking`。

建议策略：

- 将 `MemoryProvider` 的默认实现迁移到 async 内部 helper：`load_memory_async`、`save_memory_async`、`append_history_async`。
- 保留现有同步 public API 时，明确只用于同步调用和测试；async trait 方法不要调用同步写路径。
- 写文件使用 `tokio::fs::create_dir_all`、`tokio::fs::write`。需要可靠性时与 P0-3 原子持久化方案合并：写临时文件、flush/fsync、rename。
- 对 `HISTORY.md` 使用 append-only `tokio::fs::OpenOptions::append(true).create(true)`，并为同一 memory 文件增加 per-path `tokio::sync::Mutex`，避免并发追加交错或丢写。
- 对必须使用阻塞库的路径，用 `tokio::task::spawn_blocking` 包住整个阻塞闭包，不要在 async worker 上直接执行。

## 与 pro 分支的差异说明

`branch-ownership.md` 将 P2-10 标记为 both：`agent_loop.rs` 在 pro 中差异极大，clone 热点在 pro 扩展后更明显；`memory/manager.rs` 同步 IO 两边都有。

main 的修复应先集中在共享基础设施：默认 MemoryManager async 化、tool definition 缓存、事件 payload 截断、减少主循环里明显的大对象 clone。pro 需要在自己的扩展主循环、GUI/规划/工具链路径上单独定位 clone 热点，不能假设 main 的行号和结构可直接套用。

## 影响评估

- 长会话和多工具调用时，事件与 debug payload 会复制大量字符串和 JSON。
- 大工具输出会在事件总线中完整复制，增加内存峰值和 UI/manager 传输压力。
- `MemoryManager::append_history` O(file size) 重写历史文件，历史越大，每轮成本越高。
- async trait 内同步文件 IO 会阻塞 Tokio worker，影响 provider stream、channel 消息处理和其他后台任务。
- 并发 memory sync 时，读改写的 `HISTORY.md` 存在丢写风险。

## 解决方案

建议按影响面从小到大推进。

第一阶段：低风险减 clone。

- 在 `ToolRegistry` 中缓存 tool definitions，暴露 revision；`loop_turn.rs` 每轮复用 `Arc<[serde_json::Value]>`，cron 场景可缓存一份 without-cron definitions。
- `AgentEvent::ToolCallFinished` 改为携带 `result_preview`、`result_bytes`、`is_truncated`；完整结果仅写入受控 debug raw 或落盘 trace。
- event 发布前将 `channel`、`chat_id` 提前绑定为局部引用或一次性 clone，避免 streaming delta 分支重复 clone。
- tool 参数 preview 从已 materialized 的 `serde_json::Value` 生成，避免 `to_string` 后又 clone preview。

第二阶段：MemoryManager async 化。

- 为 `MemoryManager` 增加 async helper，并在 `MemoryProvider::sync_turn` 中调用 async helper。
- `save_memory_async` / `save_daily_note_async` 使用 `tokio::fs`，并复用 P0-3 的原子写 helper。
- `append_history_async` 改为 append-only，并使用 per-file async mutex 串行化同一文件追加。
- `load_memory` 在 system prompt 路径可改为 async，或在 context builder 中通过 `spawn_blocking` 临时隔离，直到接口完全 async 化。

第三阶段：配置和消息结构共享。

- 将大配置快照改为 `Arc<T>` 或 `ArcSwap`，尤其是 `mcp_servers`、network config、tool definitions。
- provider API 若允许，优先接受 `Vec<Message>` 的 owned 请求但减少上游 clone；若未来要继续优化，可评估 `Arc<[Message]>` 或 borrowed request，但这会影响 provider trait，需单独设计。
- 对 session/memory 持久化与 P0-3 合并设计：异步、原子、可追加、可 compaction。

## 验证方法

功能验证：

```powershell
cargo test -p agent-diva-core memory
cargo test -p agent-diva-agent agent_loop
just fmt-check
just check
```

性能/资源验证建议：

- 构造包含大 `HISTORY.md` 的临时 workspace，连续调用 `MemoryProvider::sync_turn`，确认追加耗时不随历史文件线性增长。
- 构造大工具输出，验证事件 payload 被截断，LLM context 中的 tool result 仍保持现有截断语义。
- 并发触发多个 `sync_turn`，确认 `HISTORY.md` 不丢条目、不交错破坏格式。
- 在 debug logger 开关关闭时，确认不会构造完整 raw/debug payload。
- 对长 streaming response 做 smoke，确认 delta 事件仍按预期发布。

## 优先级

P2。该问题通常不会直接导致权限突破或立即崩溃，但会随着长会话、大工具输出、频繁 memory 写入和高并发 channel 使用逐步放大，最终表现为响应变慢、worker 被阻塞、内存峰值升高和历史文件丢写。建议在 P0/P1 可靠性问题之后处理，并与 P0-3 原子持久化合并规划。
