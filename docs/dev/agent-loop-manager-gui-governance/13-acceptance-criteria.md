# 验收标准

## 1. 架构验收

- [ ] 没有新增第二套 kernel、runtime、Manager API、store 或 Tool Gateway。
- [ ] AgentLoop 仍是唯一 agent turn 入口。
- [ ] 所有 turn 内 tool execution 经过一个可定位 seam，并继续复用现有 policy/sandbox。
- [ ] Manager handler 不构造 AgentLoop/provider/store，不直接推导 terminal domain state。
- [ ] GUI 业务能力与 Tauri LOCAL 能力可由清单和目录结构区分。
- [ ] GUI terminal state 来自 Manager/domain projection。

## 2. Agent Loop 验收

- [ ] `process_inbound_message_inner` 成为薄编排；阶段职责分文件。
- [ ] turn 起点、tool 后、runtime-control 后使用同一 policy phase 决议。
- [ ] 普通 chat、tool call、Plan、stop、overflow、session save 行为等价。
- [ ] effectful denial 在 tool 调用前发生且无副作用。
- [ ] cancel/timeout/unknown outcome 不触发不安全自动重试。
- [ ] 每个阶段有成功、失败和竞态测试。

## 3. Manager 验收

- [ ] `handlers.rs` 只做 module/re-export，不再容纳多域实现。
- [ ] `state.rs` 只保留 AppState/兼容 re-export，DTO 按域组织。
- [ ] 现有 `/api/*` path、method、status 和字段兼容。
- [ ] SSE 事件映射集中且有顺序测试。
- [ ] `Manager` 是 façade，runtime bootstrap 是唯一 composition root。
- [ ] 新 handler 使用 typed error，不返回假成功。

## 4. GUI/Tauri 验收

- [ ] UI 只从一个 API barrel 导入业务 API。
- [ ] component、view、composable 不新增裸 `invoke`。
- [ ] 每个 capability 标为 `MANAGER/LOCAL/DEFERRED/REMOVED`。
- [ ] `DEFERRED/REMOVED` fail-closed，不返回空成功。
- [ ] `commands.rs` 拆为 LOCAL 与 Manager proxy/event bridge。
- [ ] `App.vue` 只做 shell/composition，不拥有 DTO parsing 和 transport。
- [ ] Plan/session/chat stream 的 authoritative projection 与 in-flight 状态分离。
- [ ] desktop pet、tray、prefs、gateway lifecycle 真实 smoke 通过。

## 5. 兼容与安全验收

- [ ] 无配置或存储 schema 变化；若有则已拆出本计划。
- [ ] 无双写、长期 feature flag 或 legacy/new runtime 并存。
- [ ] 非测试代码无新增 `unwrap/expect/unsafe`。
- [ ] secret、prompt、附件正文和敏感工具输出未进入日志/fixture。
- [ ] stale revision、disconnect、late event、refresh failure 均 fail-closed。

## 6. 质量验收

- [ ] AgentLoop/Manager/GUI characterization 和 contract tests 通过。
- [ ] `just fmt-check`、`just check`、`just test` 通过或已记录与本次无关的基线 blocker。
- [ ] GUI Vitest、production build、Tauri smoke 通过。
- [ ] 首字/tool roundtrip p95 退化不超过 10%，startup 不超过 15%。
- [ ] listener/task 不随 session 切换泄漏。
- [ ] 每个 story 有 iteration log、明确 rollback 和聚焦提交。

## 7. 结构目标

以下是 review 阈值，不是用机械切文件作弊的 KPI：

- [ ] `loop_turn.rs` 不再包含完整业务实现，目标不超过约 500 行。
- [ ] 单个新 turn stage 文件目标不超过约 600 行。
- [ ] Manager 单 domain handler 目标不超过约 500 行。
- [ ] `App.vue` script 目标不超过约 600 行。
- [ ] Tauri 单 command domain 目标不超过约 500 行。
- [ ] 超阈值文件有明确单一职责与书面例外。

## 8. 文档验收

- [x] 13 个设计维度齐全。
- [x] deep 分支的可复用原则与拒绝移植项已区分。
- [x] 当前代码引用包含路径和行号。
- [x] 研究完成与实现授权明确分离。
