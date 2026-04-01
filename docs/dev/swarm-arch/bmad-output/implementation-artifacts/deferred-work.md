# Deferred work (BMad code review)

## Deferred from: code review of 5-3-handoff-state-checkpoint.md (2026-04-01)

- **FullSwarm 且无过程管道时不跑序曲：** `process_inbound_message_inner` 仅在 `Some(pipe)` 时调用 `run_swarm_deliberation_prelude`，与 FR22 / `preludeLlmCalls` 文档一致；非 5.3 新增缺口。

## Deferred from: code review of 6-5-mig-capability-registry-subagent-tools.md (2026-04-01)

- **子代理提示与联网工具注册不一致**：`build_subagent_prompt` 固定描述可搜索/抓取网页；`network.web.*.enabled` 为 false 时 registry 不含对应工具。若需与工具表严格一致，可按 `NetworkToolConfig` 或 catalog 生成提示文案（另开故事或纳入后续 MIG）。
- **迭代上限边界的用户可见文案**：`max_iterations` 用尽且仍在 tool 循环时返回通用完成提示；若需区分「被截断」与「正常完成」，可在子代理循环层显式状态（非 6.5 范围）。

## Deferred from: code review of 6-4-cortex-gui-gateway-parity.md (2026-03-31)

- **`POST /api/swarm/cortex` 路由往返单测**：`build_app` 测试现覆盖 `GET` 与 mock 通道；可补充 `POST` + JSON body 与响应体与 `CortexState`（camelCase）一致性的 axum `oneshot` 断言，与故事「单元/路由测试」表述完全对齐。

## Deferred from: code review of 6-3-doctor-capability-registry-wiring.md (2026-03-31)

- **Gateway HTTP doctor 与 CLI 的磁盘刷新边界**：仅修改工作区 `capability-manifest.json` 而不调用 `POST /api/capabilities/manifest` 且不重启 gateway 时，进程内 registry 不会自动与磁盘对齐；与分离进程架构一致，可在运维/架构说明中强调。

## Deferred from: code review of 5-2-swarm-telemetry-unified-fr22.md (2026-03-31)

- **取消 turn 无 RunTelemetry**：用户取消生成时 `process_inbound_message_inner` 提前 `return Ok(None)`，不发射 `AgentEvent::RunTelemetry`；与 Story 5.2 范围外既存行为一致，若产品需要可另开故事补「取消/中断」遥测或占位事件。

## Deferred from: code review of 6-1-convergence-timeout-observable.md (2026-03-31)

- **墙钟采样粒度与同步循环**：`execute_full_swarm_convergence_loop` 在每轮开头检查 `Instant::elapsed()`；若 `is_done` 回调长时间阻塞，墙钟无法在阻塞期间抢占。序曲/异步 LLM 超时由调用方包 `timeout`（模块文档与 README 已说明）；若未来需要「可中断的慢 `is_done`」，需另行设计（例如异步化或协作式取消）。

## Deferred from: code review of 5-1-swarm-prelude-config.md (2026-03-31)

- **同分支 diff 与 5.1 边界**：`loop_turn.rs`、`lib.rs` 等与 Story 5.1 File List 重叠的变更中混有执行层、轻量路径、收敛等非序曲能力；严格按 5.1 验收时应以 File List 与序曲/配置逻辑为准；后续可考虑拆 PR 便于回滚与评审。

## Deferred from: code review of 2-3-process-feedback-strip.md (2026-03-31)

- **皮层 UI 与 gateway 过程门控可能不同步**：条带随桌面 `cortexLayerOn` 隐藏，后端 `ProcessEventPipeline` 仍可能随 gateway 内 `CortexRuntime` 门控；与 Story 2.2/后续 gateway 同步任务收敛。

## Deferred from: code review of 4-4-doctor-hooks.md (2026-03-31)

- ~~**Doctor 与 registry 接线**~~ — **已收口（Story 6.3，2026-03-31）：** `swarm_cortex_doctor_section_for_diagnostics`（`agent-diva-agent`）；gateway 持 `Arc<PlaceholderCapabilityRegistry>`、启动加载与 GUI 同路径 `{workspace}/.agent-diva/capability-manifest.json`；`GET /api/diagnostics/swarm-doctor`、`POST /api/capabilities/manifest`；CLI `config doctor --swarm` 在无进程内 registry 时读同一文件；GUI 提交成功后 best-effort 持久化该文件。

## Deferred from: code review of 4-1-person-narrative-regression.md (2026-03-31)

- **locale 文件 diff 与故事边界**：当前分支上 `en.ts` / `zh.ts` 相对 `HEAD` 的变更包含 cortex、neuro、capability 等多块文案；评审 Story 4.1 时宜以故事 File List 及 FR8/FR9 相关片段为准，避免将其它史诗的文案改动误记为 4.1 缺陷。

## Deferred from: code review of 4-2-skills-capability-ui.md (2026-03-31)

- **Vitest 未覆盖 `CapabilityManifestPanel`**：仅 `CapabilityManifestErrorsDisplay` 有单测；后续可用 invoke mock 覆盖提交成功/失败与摘要刷新。
- **`clear()` 静默失败与占位 registry**：写锁失败时不清空；与 `submit` 路径中先 clear 再 register 组合时，可能残留旧 capability 或难以诊断；与 1-6 review 中「Poisoned RwLock 时查询静默」同类，生产化时可统一为 `Result` 或原子替换 API。

## Deferred from: code review of 1-8-convergence-policy-fr20.md (2026-03-31)

- ~~**`Timeout` 终局未在收敛循环中生成**~~ — **已收口（Story 6.1，2026-03-31）：** `ConvergencePolicy::wall_clock_timeout` + `execute_full_swarm_convergence_loop` 内墙钟检查；单测与 `run_minimal_turn_headless_with_full_swarm_events` 集成断言 `swarm_run_finished` / `Timeout`。

## Deferred from: code review of 1-7-light-path-routing-fr19.md (2026-03-31)

- ~~**Light 路径上限契约与桩分支**~~ — **已收口（Story 6.2，2026-03-31）：** `AgentLoop` 在 `ExecutionTier::Light` 下按 `LIGHT_PATH_MAX_INTERNAL_STEPS` / `LIGHT_PATH_MAX_WALL_MS` enforcement；`format_light_path_stop_for_user` + 集成测 `light_path_internal_step_cap_returns_user_message`。

## Deferred from: code review of 1-6-capability-v0-validation.md (2026-03-31)

- **`Deserialize` 绕过校验入口**：`ValidatedManifest` / `ValidatedCapability` 导出 `Deserialize` 时调用方可跳过 `parse_and_validate_*`；v0 可接受，建议在模块文档注明须走验证 API。
- **Poisoned `RwLock` 时查询静默**：占位 registry 的 `len` / `ids` 在锁中毒时返回 0/空；生产化时可改为 `Result` 或明确文档约束。

## Deferred from: code review of 1-4-cortex-off-headless-tests.md (2026-03-31)

- `full_swarm_cap_observable_via_pipeline_without_gui`（`minimal_turn.rs`）未列入 `CORTEX_OFF_SIMPLIFIED_MODE.md` §4 测试对照表且无 `doc-ref:`；与 Story 1.8（FR20）文档对齐时可补一行对照或注释，避免 AC#3 可追溯性在跨 story 边界显得不完整。

## Deferred from: code review of 1-5-min-process-events-ui.md (2026-03-31)

- `ProcessEventBatchSink::deliver_batch` 同步实现若阻塞会拖慢 AgentLoop；文档与 trait 注释已要求轻量与非 UI 线程阻塞，生产集成（如 Tauri emit）需自行遵守。

## Deferred from: code review of 1-1-swarm-crate-workspace.md (2026-03-30)

- README 外链 `../../agent-diva-swarm/docs/ARCHITECTURE_DESIGN.md` 依赖 newspace 并排 monorepo；单仓 clone `agent-diva` 时可能失效 — 可选补充 README 前提说明。
- Story File List 与当前 crate 文件树不完全一致（后续故事代码已合入）；可选更新 story 文档以免审计混淆。
