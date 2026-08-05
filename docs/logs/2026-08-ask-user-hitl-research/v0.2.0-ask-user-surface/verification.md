# Verification — ask_user 表面闭环（Phase 2）

- 版本：`v0.2.0-ask-user-surface`
- 日期：2026-08-05

## 命令与结果（规则：post-dev-stage-validation）

| 命令 | 结果 |
|------|------|
| `cargo test -p agent-diva-manager --lib handlers::ask_user` | 7 passed（list/answer/other/404/422/cancel/expired） |
| `cargo test -p agent-diva-manager` | 106 passed |
| `cargo test -p agent-diva-cli --lib ask_user_answerer_tests` | 2 passed（headless 自动 cancel、残留清理） |
| `cd agent-diva-gui && npm run test -- --run` | 58 files / 451 tests passed |
| `npx vue-tsc --noEmit`（agent-diva-gui） | 通过 |
| `cargo check -p agent-diva-gui`（src-tauri） | 通过 |
| `just fmt-check` / `just check`（clippy -D warnings） | 通过（workspace 全量） |
| 受影响 crate 全量测试 | core 676 / tools 94 / agent 371（+集成 28）/ manager 106 / CLI answerer 2 全过，无回归 |

## 验收矩阵对照（提案 §8 Phase 2）

| 项 | 结果 |
|----|------|
| Manager 暴露挂起问题与回答回传 | 通过（3 端点 + handler 测试） |
| GUI 问题卡（选项/Other/取消） | 通过（组件测试 + vue-tsc + 451 全量） |
| CLI interactive（Select/Input） | 通过（headless 单测 + 代码路径）；终端实测见 smoke |
| 人工 smoke：互动调研出现 ask_user tool call | **需真实 LLM 会话验收**（见 acceptance.md 步骤） |
| headless 不挂死 | 通过（answerer 自动 cancel 单测） |

## 说明

- `smoke-test-required-for-user-visible-change`：本迭代以 mock/组件/集成测试作为
  自动化门；真实 LLM 触发的 CLI/GUI 互动调研 smoke 需要配置好的 provider，
  步骤记录于 `acceptance.md`，在可用环境下执行后回填本文件。
- 全量 `just test` 的 6 个 CLI wiremock 502 失败为预存在问题
  （TODOLIST `CLI-WIREMOCK-502-PREEXISTING`，stash 验证与本迭代无关）。
