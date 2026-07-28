# 测试策略

## 1. 原则

本治理的首要测试目标是 **行为等价**。结构变好但事件顺序、工具权限、session 持久化或 GUI 恢复发生变化，均视为失败。

## 2. Agent Loop characterization

至少覆盖：

1. 无工具的普通多轮对话；
2. 单/多 tool call，成功、错误、超时；
3. Plan 模式只读、AwaitingApproval 禁止副作用；
4. approval 后下一个 iteration 工具集合刷新；
5. terminal plan 不锁死普通 chat；
6. runtime tool/MCP 配置更新后仍应用 phase policy；
7. stop 与 tool start/finish 的竞态；
8. context overflow 后压缩/重试；
9. token ledger、session save、memory sync 各自失败；
10. provider 返回空内容、内部 protocol marker、iteration limit。

测试应优先落在 `agent-diva-agent/tests/`；纯 phase/summary helper 可保留内联单测。

## 3. Manager contract tests

对 [server.rs](../../../agent-diva-manager/src/server.rs:1) 的路由建立表驱动测试：

- method/path/status/content-type；
- 成功 DTO fixture；
- invalid input、not found、conflict、internal error；
- SSE event 名称、request/session/trace 关联和顺序；
- handler 不直接构造 AgentLoop/store 的结构审查。

拆 handler 时必须做到旧测试原样通过；只有测试本身依赖私有路径时才允许机械迁移。

## 4. GUI tests

### API contract

- 每个 domain adapter 有成功、transport failure、invalid payload 测试；
- `DEFERRED/REMOVED` capability fail-closed；
- Rust fixture 可被 TypeScript guard 解析；
- `desktop.ts` re-export 与新 barrel 在迁移期等价。

### composable

- session cache 失效后以 server 为准；
- stream event 乱序、重复、stop 后迟到；
- approval revision stale；
- mutation 成功但 refresh 失败；
- health 从 unavailable 恢复后 reload；
- local placeholder 不成为 terminal fact。

### 组件

组件测试只验证显示和 intent，不 mock 内部 domain 状态机。

## 5. Smoke 矩阵

| 场景 | CLI | Manager | Tauri GUI |
| --- | --- | --- | --- |
| 普通 chat | 必须 | 必须 | 必须 |
| tool call | 必须 | 必须 | 必须展示 |
| stop | 必须 | 必须 | 必须 |
| plan approve/deny | 必须 | 必须 | 必须 |
| session restart | 必须 | 必须 | 必须 reload |
| gateway debug/release | 不适用 | 外部启动 | 两种模式 |
| pet/local prefs | 不适用 | 不适用 | 必须 |

## 6. 每阶段验证命令

按影响范围选择，最终阶段必须包含：

```text
cargo test -p agent-diva-agent
cargo test -p agent-diva-manager
cargo test -p agent-diva-gui
cd agent-diva-gui && npm test
cd agent-diva-gui && npm run build
just fmt-check
just check
just test
```

当前仅交付文档，不修改 Rust/Vue 行为，因此本轮只运行 Markdown/链接/差异校验；上述命令是后续实施门禁。

## 7. G0–G5 可执行 QA 表

| 阶段 | 工具/命令 | 执行步骤 | 预期结果 |
| --- | --- | --- | --- |
| G0 AgentLoop 基线 | `cargo test -p agent-diva-agent policy_phase_prioritizes_explicit_plan_mode_and_releases_terminal_plans`；新增 characterization suite | 分别预置无 Plan、AwaitingApproval、approved Execute、Completed；发送普通/plan-mode 消息；在 provider/tool fake 中记录调用 | terminal Plan 不锁死；plan-mode/AwaitingApproval 无副作用；批准后下一 iteration 能看到 Execute 工具；事件顺序形成冻结 fixture |
| G0 Manager 基线 | `cargo test -p agent-diva-manager chat_plan_update_forward`；`cargo test -p agent-diva-manager build_router_keeps_health_and_skills_routes_without_overlap` | 对当前 route 表发 success/invalid/not-found；向 bus 注入 plan/tool/chat events 并收集 SSE | path/method/status/DTO 不变；SSE event 名、request/session 关联和相对顺序固定 |
| G0 GUI 基线 | `cd agent-diva-gui && npm test`；`npm run build` | 用现有 App/Chat/Plan tests 记录 send、tool row、stop、approval、session restore；保存 capability invoke 清单 | 当前用户旅程和 invoke/listen 面有可重复基线；已知失败单独登记，不将其误算为本次回归 |
| G1 AgentLoop 拆分 | `cargo test -p agent-diva-agent`；新增 `turn_pipeline` integration tests | fake provider 依次产生 text、tool call、tool error、overflow；在 tool start 后触发 cancel；模拟 session/token/memory 单点失败 | 阶段拆分前后 outbound、persisted session、tool 调用次数和 event sequence 等价；denial 在调用前；cancel 无迟到副作用 |
| G2 Manager 薄化 | `cargo test -p agent-diva-manager`；`cargo test -p agent-diva-manager chat_plan_update_forward -- --nocapture` | 对拆出的每个 domain 重放 G0 route/DTO fixture；断开 SSE client 后重连；模拟 service conflict/not-ready | HTTP 契约不变；handler 仅做 decode/delegate/map；重复/断线不伪造 committed fact；typed error 映射稳定 |
| G3 capability fail-closed | `cd agent-diva-gui && npm test -- src/api`；`rg -n \"invoke\\(\" src --glob \"*.ts\" --glob \"*.vue\"` | 对每个 capability class 调用 adapter：MANAGER 使用 fake transport，LOCAL 使用 fake Host，DEFERRED/REMOVED 直接调用 | MANAGER/LOCAL 只命中指定 transport；DEFERRED/REMOVED 返回明确 `not_available/removed`；组件和 composable 中无新增裸 `invoke` |
| G3 Tauri command 拆分 | `cargo test -p agent-diva-gui`；`cargo test -p agent-diva-gui embedded_gateway_serves_health_endpoint -- --nocapture` | 重放 Manager proxy payload；对 LOCAL path 做 config-dir 内/外测试；验证 command 注册清单 | command 名/payload 不变；业务 command 不执行本地 domain 逻辑；LOCAL path 越界 fail-closed；health 测试通过 |
| G4 GUI state | `cd agent-diva-gui && npm test -- src/App src/components/ChatView src/components/planning`；`npm run build` | 注入重复/乱序 delta、stop 后迟到 finish、stale approval、mutation 成功但 refresh 失败、cache 旧于 server | reducer 幂等；stop 不复活 turn；stale 要求刷新；不重复提交副作用；server projection 覆盖 cache/local terminal state |
| G4 Tauri debug smoke | 终端 A：`just diva-gate`；终端 B：`cd agent-diva-gui && npm run tauri dev` | 等待 connected；完成 chat→tool→stop→session reload→plan approve/deny；打开 pet/prefs | debug 模式只连接外部 gateway；主旅程可用；关闭窗口后无遗留 listener/task；LOCAL 能力不经过 Manager domain |
| G4 Tauri release smoke | `cd agent-diva-gui && npm run tauri build`，启动产物时确保无外部 gateway | 首次启动等待 embedded gateway；重复 debug smoke；退出后检查 gateway 子任务/端口 | release 自行启动动态端口 embedded gateway；GUI health 恢复；完整退出后端口释放、无孤儿进程 |
| G5 契约/收尾 | Rust fixture tests；`cd agent-diva-gui && npm test -- src/api`；`just ci` | Rust 生成/保存 DTO fixture，TS guard 逐一解析；`rg` 查旧 DTO、旧 re-export、dead command；执行全量 smoke | required fields/enum/tag 一致；无重复 authority/transport；兼容壳确实无引用；CI、CLI/Manager/Tauri smoke 全绿 |

若测试名因 G1/G2 移动而变化，必须在同一提交更新本表和 iteration `verification.md`；不得用宽泛的“跑全部测试”替代场景证据。

## 8. 非功能测试

- 记录 chat 首 token、tool roundtrip、session reload 的基线与回归；
- 对 100 个 session、长 transcript、连续 tool events 做负载 smoke；
- 对 cancel、disconnect、restart 做故障注入；
- 检查日志不泄漏 prompt、token、API key 或用户文件内容。
