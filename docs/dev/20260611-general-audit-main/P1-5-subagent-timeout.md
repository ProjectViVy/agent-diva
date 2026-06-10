# P1-5: 子代理 timeout/并发治理缺口

## 问题描述

审计报告将 P1-5 归类为 both：main 和 pro 都需要按各自实现独立修复。需要注意的是，当前 main 分支的 `agent-diva-agent/src/subagent.rs` 已经不是“完全无 timeout/并发上限”的状态，但治理仍不完整。

当前 main 代码里已有的防护：

- `agent-diva-agent/src/subagent.rs:57` 保存 `running_tasks: Arc<tokio::sync::Mutex<HashMap<String, JoinHandle<()>>>>`，用于记录后台子代理任务。
- `agent-diva-agent/src/subagent.rs:59` 保存 `concurrency_limit: Arc<Semaphore>`。
- `agent-diva-agent/src/subagent.rs:93` 用 `Semaphore::new(subagent_policy.max_concurrent)` 初始化并发上限。
- `agent-diva-agent/src/subagent.rs:124` 在 `spawn` 中用 `try_acquire_owned()` 立即拒绝超过并发上限的子代理。
- `agent-diva-agent/src/subagent.rs:327-331` 在子代理执行循环中创建 `LoopGuard::new(DEFAULT_SUBAGENT_MAX_ITERATIONS, DEFAULT_SUBAGENT_LOOP_TIMEOUT, ...)`。
- `agent-diva-agent/src/loop_guard.rs:5` 定义 `DEFAULT_SUBAGENT_LOOP_TIMEOUT = 120s`，`agent-diva-agent/src/loop_guard.rs:6` 定义 `DEFAULT_SUBAGENT_MAX_ITERATIONS = 15`。
- `agent-diva-agent/src/subagent_policy.rs` 通过 `SubagentPolicy` 承接 core 配置；`agent-diva-core/src/config/schema.rs:968-1003` 默认 `max_concurrent = 2`、`max_depth = 1`。

仍存在的 main 分支问题：

1. timeout 不是任务级强制取消。`LoopGuard::check_elapsed()` 只在迭代开始、provider 返回后、工具调用前后等显式检查点生效。如果 `provider.chat(...).await`、`tools.execute(...).await` 或底层网络/进程调用长期不返回，120 秒预算不会主动中断当前 await。
2. 没有公开取消/abort API。`SubagentManager::spawn` 返回用户可见 task id，但 `SubagentManager` 只有 `get_running_count()`，没有 `cancel_subagent(task_id)`、`cancel_all()` 或 shutdown hook。后台任务只能自然结束。
3. `JoinHandle<()>` 不被 await。`agent-diva-agent/src/subagent.rs:163` 用 `tokio::spawn` 创建任务，`agent-diva-agent/src/subagent.rs:193-195` 将 handle 存入 map，但没有监控 join 结果。panic、取消、运行时错误无法结构化上报。
4. `running_tasks` 仍有插入/清理竞态。当前流程是先 `tokio::spawn`，任务结束后在任务内部 `remove(&task_id_clone)`，然后外层再把 `JoinHandle` 插入 map。若任务极快完成，内部清理可能先于外层插入发生，导致已结束任务的 handle 被插入并长期残留，`get_running_count()` 失真，并发 permit 已释放但 running map 仍显示运行中。
5. 并发上限仅覆盖已成功获取 permit 的后台任务，不解决任务生命周期管理。`OwnedSemaphorePermit` 被移动到 `run_subagent`，任务结束会释放 permit；但因为没有 cancel/shutdown API，进程关闭或用户撤销任务时无法优雅释放并确认完成。
6. `build_identity_summary` 在 `agent-diva-agent/src/subagent.rs:506` 使用 `std::fs::read_to_string`，虽然文件较小，但它位于子代理执行路径内，仍会占用 Tokio worker。

## 与 pro 分支的差异说明

`branch-ownership.md` 将 P1-5 标记为 both，原因是 `subagent.rs` 两边都有，但 pro 已大幅改写并加入部分 timeout 逻辑；main 的修复无法直接 merge 到 pro。

main 当前重点是补齐“任务级治理”：强制 timeout、取消 API、join 结果观测、running map 注册竞态。pro 的重点应按 pro 现有子代理架构补齐并发上限和任务 registry，不能直接照搬 main 的 `Semaphore + running_tasks` 结构。

## 影响评估

- 长时间卡住的 provider/tool await 会让子代理一直占用并发 permit，后续子代理被拒绝。
- 用户拿到 task id 后无法取消任务，只能等待自然结束或重启进程。
- running task map 可能残留已结束任务，导致状态观测不可信。
- 子代理 panic 不会形成业务事件，排障只能依赖运行时日志。
- 子代理可执行 shell/filesystem 等工具，缺少强制取消会放大外部命令、网络调用和文件操作的资源占用风险。

## 解决方案

针对 main 分支建议分三步修复。

第一步，改造任务注册顺序，消除 running map 竞态：

- 在 `spawn` 分配 `task_id` 后，先向 `running_tasks` 插入一个任务记录占位，而不是 spawn 后再插入。
- 将 map value 从裸 `JoinHandle<()>` 升级为 `SubagentTaskRecord`，至少包含 `label`、`started_at`、`origin_channel`、`origin_chat_id`、`JoinHandle<()>`、取消 token 或状态字段。
- 如果保持 `JoinHandle` 必须后填充，使用单次临界区完成“注册记录 + 写入 handle”，避免任务内部 remove 早于外层 insert。

第二步，引入强制 timeout 和取消：

- 使用 `tokio_util::sync::CancellationToken` 或内部 `watch` channel，传入 `run_subagent`、`execute_subagent_task` 和工具执行上下文。
- 在 `run_subagent` 外层包 `tokio::time::timeout(subagent_policy.timeout_or_default, execute_future)`。timeout 后返回明确的 `SubAgent timed out` 结果并触发公告。
- 对 provider 调用和工具调用增加局部 timeout，例如 `tokio::time::timeout(provider_timeout, provider.chat(...))`、`tokio::time::timeout(tool_timeout, tools.execute(...))`。不要只依赖 `LoopGuard` 的检查点。
- 为外部工具保留现有 `exec_timeout`，但将它与子代理全局 timeout 区分：`exec_timeout` 控制单个命令，subagent timeout 控制整体任务。

第三步，补齐管理 API 和可观测性：

- 增加 `SubagentManager::cancel_subagent(&self, task_id: &str) -> Result<()>`：从 registry 取出记录，触发 cancellation token，等待一个短 timeout，超时后 `JoinHandle::abort()`，再 await join 结果。
- 增加 `cancel_all()` 或 `shutdown()`，用于 AgentLoop 停止时清理后台子代理。
- 子代理任务结束后记录状态：completed、failed、timed_out、cancelled、panicked。
- 对 `JoinError::is_panic()` 和 `JoinError::is_cancelled()` 做结构化日志和事件上报。
- `get_running_count()` 应只统计状态为 running 的记录；必要时补一个 `list_running_tasks()` 供调试和控制面使用。
- 将 `build_identity_summary` 的同步读取移到 `tokio::fs`，或在构造子代理 prompt 前通过 `spawn_blocking` 读取，避免在异步 worker 上做同步文件 IO。

建议新增或调整配置：

- `tools.subagent.max_concurrent`：保留现有默认 2。
- `tools.subagent.max_depth`：保留现有默认 1。
- 新增 `tools.subagent.timeout_secs`：默认可取 120，与当前 `DEFAULT_SUBAGENT_LOOP_TIMEOUT` 对齐。
- 新增 `tools.subagent.cancel_grace_secs`：取消后等待优雅退出的时间，例如 5 秒。

## 验证方法

单元测试建议：

- 构造永不返回的 mock provider，验证 `spawn` 后任务会在 `timeout_secs` 内结束并释放 permit。
- 构造阻塞 tool，验证工具级 timeout 生效，并且最终公告为 timeout/error。
- 构造快速完成的 provider，循环 spawn 多次，验证 `running_tasks` 不残留已完成任务。
- `max_concurrent = 1` 时，第一任务阻塞、第二任务应被拒绝；取消第一任务后，第三任务应可启动。
- 触发任务 panic，验证 join 结果被记录为 panicked，running map 被清理。

执行命令：

```powershell
cargo test -p agent-diva-agent subagent
just fmt-check
just check
```

如果接入控制面取消 API，还需要增加 CLI/manager smoke：启动一个长运行子代理，调用取消命令，确认任务列表清空并且用户侧收到取消或失败通知。

## 优先级

P1。当前 main 已具备默认并发上限和循环级超时，风险低于原始“完全无 timeout/并发上限”的描述；但缺少强制取消、join 观测和 registry 原子性，仍会造成资源占用不可控和状态不可信，应在高优先级并发稳定性阶段修复。
