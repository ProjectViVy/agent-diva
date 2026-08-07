# LAPUTA-COGNITIVE-SYNC 验证记录

## 门禁纪律

每切片提交前执行 `cargo fmt --all` + `cargo clippy --workspace -- -D warnings` +
受影响 crate 测试；全部切片完成后跑全量 `just ci`。

## 全量 `just ci` 最终结果

- fmt / clippy（-D warnings）：通过。
- workspace 测试：除基线已知失败外全绿。
- 基线失败（开工前即存在，S0 已核实）：`agent-diva-cli` 6 个 wiremock 502
  用例（记为 `CLI-WIREMOCK-502-PREEXISTING`，TODOLIST 有案）：
  - `approval_commands::tests::command_plan_and_memory_decisions_share_one_cli_contract`
  - `approval_commands::tests::explicit_allow_once_uses_version_and_idempotency_key`
  - `approval_commands::tests::unified_list_parses_all_three_domains`
  - `chat_commands::approval_mode_tests::default_headless_cancels_high_risk_memory_pending`
  - `chat_commands::approval_mode_tests::explicit_queue_rejects_command_because_raw_payload_is_not_durable`
  - `chat_commands::approval_mode_tests::explicit_queue_returns_plan_pending_without_waiting_for_completion`

最终 5 个核心 crate 全量测试（收尾复跑）：0 failed ——
core 690、agent 382、manager 109、autodream/laputa 及其集成套件全部通过，
含新增 `context_plane_invariants` 8/8。

## 分切片验证

| 切片 | 关键验证 |
|------|----------|
| S0 | `just ci` 恢复至基线（仅上述 6 个预存在失败） |
| S1 | 种子幂等、缺失兜底、既有文件不覆盖单测 |
| S2 | claim 解析/渲染往返、投影 scope+budget、confirmed+user 保护（仅 stale+备注）、ledger 审计 |
| S3 | 会话内写入不影响本会话快照；新会话 capture 可见 |
| S4 | 迁移 proposal 审批路径、`.laputa/legacy/` 存档、负向回归：装配不再读取退役人格文件 |
| S5 | 已删 section/proposal 名字：FromStr/serde/routing/写入全部稳定失败；落盘历史 `list_proposals` 容错跳过 |
| S6 | rhythm patch proposal 删除后路由稳定失败（`UnknownProposalType` ×5）；启动注入无 `## Rhythm Signals`；D2 迁移幂等 + 冲突保留 legacy 副本 |
| S7 | `agent-diva-laputa/tests/context_plane_invariants.rs` 8/8 |

## S7 不变量矩阵（8 行，负向回归）

测试文件：`agent-diva-laputa/tests/context_plane_invariants.rs`。
方法：向每层存储写入唯一 marker，断言 marker 不出现在任何 prompt 面。

1. MEMRULES 永不入 prompt（marker 写入 `.laputa/cognitive/MEMRULES.MD`）
2. 完整 WORLD 永不入 prompt（marker 写入 `.laputa/cognitive/WORLD.MD`）
3. 报告永不默认注入（marker 写入 `.laputa/reports/daily/2026-08-01.md`）
4. Frozen Core 会话内不变（capture 后覆盖 section，快照不动；次会话可见）
5. WORLD 投影仅 scope 匹配且预算内（20×280 字符 claim，默认 4000 预算截断；
   非匹配 scope 投影为空）
6. 已删 section 数据永不出现（残留 `history_md.json` 不进 snapshot/prompt；
   退役 `history_patch` proposal 文件被 listing 容错跳过；`history_md` 解析失败）
7. 已退役人格文件永不入 prompt（SOUL/IDENTITY/USER/BOOTSTRAP/MEMORY/HISTORY ×6 marker）
8. 证据有界（空 evidence 拒绝；仅 ContextCompaction 二级证据拒绝；主证据通过）

## S6-4 节律链路验证（如实记录）

CLI 无 autodream 入口；完整 daemon 真机需 gateway + LLM，本环境不现实。
采用进程级集成测试覆盖完整触发链：

- `cargo test -p agent-diva-autodream --test service`：notebook_daily /
  notebook_weekly / notebook_monthly trigger 与 scheduled_monthly ×2 用例通过；
- `agent-diva-manager` `runtime_clock_injection` 用例通过（monthly cron kind
  `NOTEBOOK_MONTHLY_CRON_KIND` 映射）；
- 报告产物读写路径（gui notebook.rs / autodream monthly.rs / reports.rs）
  测试已改为 `.laputa/reports/` 并通过。

完整 daemon-cron 真机验证（GUI 按钮触发 + cron 实际落盘）留待 G2D+ 桌面验收。
