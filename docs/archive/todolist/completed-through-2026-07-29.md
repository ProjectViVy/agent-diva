# TODOLIST Completed Archive

归档时间：2026-07-29。

本文件保存此前位于根 `TODOLIST.md` 的已完成项目。详细验证证据继续以各项目的 `docs/logs` 目录和 Git 历史为准。

## 2026-07-11 至 2026-07-16

- [x] **普通聊天支持 update_plan TODO 清单** 完成核心类型、工具、事件、普通聊天注册、系统提示、Manager SSE、TUI/GUI 渲染和核心测试。
  - 范围：`agent-diva-core`、`agent-diva-tools`、`agent-diva-agent`、`agent-diva-manager`、`agent-diva-cli`、`agent-diva-gui`。
  - 日志：`docs/logs/2026-07-normal-chat-update-plan/v0.0.1-normal-chat-update-plan/`
  - 验证：`cargo test --workspace update_plan` 与 `cargo clippy --workspace -D warnings`。
- [x] **P2/P3 approval boundary and runtime capability enforcement（2026-07-11）** 完成 `plan_submit`、revision-bound approval、提交/审批事件、内容修订失效、materialized TODO 保护，以及 phase-aware tool assembly/pre-call denial。
  - 日志：`docs/logs/2026-07-plan-todo-p2-p3/v0.0.1-approval-runtime-enforcement/`
- [x] **Mentle Windows gateway stack overflow 修复** 通过 Windows native-open isolation、进程默认配置、大栈 assemble thread 和 CLI PE/worker 16 MiB 栈修复启动崩溃。
  - 日志：`docs/logs/2026-07-10-mentle-windows-stack-overflow/v0.0.2-windows-native-open-isolation/`
- [x] **Workspace update_plan 编译阻塞清理** 补齐 provider/neuron/e2e/migration 调用与结构字段，使 `cargo test --workspace update_plan` 可编译并通过。
- [x] **Planning report clippy 清理** 合并 `agent-diva-core/src/planning/report.rs` 的重复条件分支；workspace clippy 通过，保留第三方 future-incompat 警告。

## 2026-07-04 至 2026-07-05

- [x] **LLM 归纳手动日报、周报、月报** 完成 fact bundle、可选 LLM curation、证据验证、确定性 fallback、Manager 注入及 GUI generation mode。
  - 日志：`docs/logs/2026-07-llm-curated-manual-reports/v0.0.2-impl-llm-curated-manual-reports/`
- [x] **GUI 宠物沉浸模式侧边栏导航修复** 排除 overlay sidebar 的 `pointer-events: none` 规则，恢复沉浸/全屏模式返回主页面。
  - Commit：`8c6fa68`
- [x] **Backlog normalization（2026-07-04）** 关闭已解决事项，并把剩余未完成项规范为显式 deferred 状态。
- [x] **Wave 0 CI stabilization**
- [x] **Plan mode runtime wiring**
  - 验证：`docs/logs/2026-06-plan-mode-runtime/v0.0.1-plan-mode-runtime-wiring/verification.md`
- [x] **Parallel lock mechanism** 引入仓库级 `LOCK.md` 协作机制。
- [x] **Wave 1 remediation** 关闭 E2E false-green、token budget、supervised-run cancellation 和 security production wiring 阻塞。
  - 日志：`docs/logs/2026-07-wave1-remediation/v0.0.1-wave1-remediation/`
- [x] **Wave 2 observability remediation** 关闭 audit sink、`/api/logs`、Manager audit filtering、cron event ordering 和早期 tool denial audit 问题。
  - 日志：`docs/logs/2026-07-wave2-observability/v0.0.1-wave2-observability-remediation/`
- [x] **Wave 3 review**
  - 日志：`docs/logs/2026-07-wave3-review/v0.0.1-wave3-summary/`
- [x] **Wave 3 workspace CLI hardening** 关闭路径穿越、active-workspace delete bypass 和 list 写副作用。
  - 日志：`docs/logs/2026-07-wave3-remediation/v0.0.1-workspace-cli-hardening/`
- [x] **Wave C readiness and audit closure**
  - 日志：`docs/logs/2026-07-wavec-remediation/v0.0.1-wavec-remediation/`
- [x] **Wave C + Wave D review closure**
  - 日志：`docs/logs/2026-07-wavecd-remediation/v0.0.1-wavecd-review-closure/`
- [x] **Wave G review**
  - 日志：`docs/logs/2026-07-waveg-review/v0.0.1-waveg-summary/`
- [x] **Wave G remediation** 关闭 provider usage fallback、retry categorization、global tool timeout、日志 retention 和 feature-gate CI 问题。
  - 日志：`docs/logs/2026-07-waveg-remediation/v0.0.1-waveg-remediation/`

## 保留在主清单的完成证据

`Deferred Review Program` 中已勾选的 wave、角色和 commit review 条目仍保留在主文件。它们是尚未完成的 review program 的执行证据，与该 program 中仍开放的 wave reports、cross-wave matrix、priority pool 和 deferred timebox 共同阅读；在整个 program 关闭前不单独迁移。

## 2026-07-29 测试健康与积压清账

- [x] **Embedded gateway 生命周期稳定化** 服务等待明确关闭信号后才进入 Manager shutdown，并对 Tokio runtime 回收设置上限；health、启动中关闭、Drop 和幂等状态测试通过。Commit：`3bc9ea45`。
- [x] **GUI 旧回归项复核** `evolution.test.ts` 与 `NormalMode.test.ts` 共 8 项通过；当前 locale 无重复顶层 `mode`，`/miku.svg` 也不再作为模块导入。
- [x] **Core supervised executor 终态竞态关闭** SQLite claim 在执行器观察前显式提交，首次 heartbeat 延迟到正常周期，终态持久化失败不再被吞掉，外部 cancel/lost 测试等待可观察的 `Running` 状态。`just fmt-check`、`just check` 和完整 `just test` 均通过。
  - 证据：`docs/logs/2026-07-supervised-executor-race/v0.0.1-supervised-run-finalization/`
- [x] **Agent compaction harness 复核** `compaction_real_test` 已使用当前 `ContextCompactor::compact` API，`cargo test -p agent-diva-agent --test compaction_real_test --no-run` 通过。
- [x] **Workspace rustfmt 复核** `cargo fmt --all -- --check` 通过，旧 `agent-diva-e2e` drift 不再存在。
- [x] **Plan/TODO P1–P3 旧评审基线归档** 旧 tool-oriented review packet 已被 revision-bound report/store/runtime closure 取代；主清单保留原文作为审计证据，但不再把其中复选框解释为当前 backlog。
