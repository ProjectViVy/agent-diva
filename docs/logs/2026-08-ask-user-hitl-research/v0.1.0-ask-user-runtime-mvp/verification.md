# Verification — ask_user 运行时 MVP（Phase 1）

- 版本：`v0.1.0-ask-user-runtime-mvp`
- 日期：2026-08-05

## 命令与结果（规则：post-dev-stage-validation）

| 命令 | 结果 |
|------|------|
| `cargo test -p agent-diva-core --lib ask_user` | 7 passed |
| `cargo test -p agent-diva-tools --lib ask_user` | 5 passed |
| `cargo test -p agent-diva-agent --lib tool_assembly` | 23 passed（含新增 5） |
| `cargo test -p agent-diva-agent --lib ask_user_tool_call_blocks_turn_until_answered` | 1 passed（mock 集成） |
| `cargo test -p agent-diva-agent --lib context` / `subagent` | 58 / 30 passed |
| `just fmt-check` | 通过 |
| `just check`（clippy -D warnings） | 通过（workspace 全量） |
| 受影响 crate 全量测试 | core 676 / agent 371（+集成 28）/ tools 94 / manager 99 全过 |
| `just test`（workspace 全量） | 6 个既有 CLI 审批 wiremock 测试失败（`502 Bad Gateway`）；`git stash` 干净树复跑同样失败，**预存在问题与本迭代无关**，已记 TODOLIST `CLI-WIREMOCK-502-PREEXISTING` |

## 验收矩阵对照（提案 §9，Phase 1 范围）

| ID | 场景 | 结果 |
|----|------|------|
| A1 | tool definitions 含 `ask_user` | 通过（装配测试 + mock provider 断言 tools 列表） |
| A2 | mock LLM 应问场景 tool_call | 通过（`ask_user_tool_call_blocks_turn_until_answered`） |
| A3 | 选项回填 | 通过（`answer_choice_returns_selected_label`） |
| A4 | Other 回填 | 通过（`answer_other_requires_allow_other`） |
| A5 | 取消不挂死 | 通过（`cancel_unblocks_with_cancelled_status`） |
| A6 | 超时不挂死 | 通过（`request_expires_after_timeout`） |
| A7 | headless unavailable | 通过（tool + 装配测试） |
| A8 | subagent 未注册 | 通过（`tool_assembly_subagent_mode_excludes_ask_user`） |
| A9 | 危险命令仍走 exec approval | 未改动 exec/审批路径；无回归 |
| A10 | 静态问卷文件 alone 不充分 | 文档层面约束（prompt 引导） |

## 说明

- Phase 1 拍板不含 GUI/CLI 卡，`smoke-test-required-for-user-visible-change` 的
  GUI/CLI 级 smoke 留 Phase 2 表面闭环后执行；本迭代以 mock 集成测试作为
  运行时闭环的最小可执行验证。
- 全量 `just test` 的 6 个 CLI 失败为预存在环境问题（wiremock 502，stash 验证），
  已记 TODOLIST `CLI-WIREMOCK-502-PREEXISTING`；本次变更不涉及 CLI 审批路径。
