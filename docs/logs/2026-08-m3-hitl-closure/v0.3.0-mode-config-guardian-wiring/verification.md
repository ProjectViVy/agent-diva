# Verification

## 命令与结果

- `cargo test -p agent-diva-sandbox` → 126 passed（含 `for_ask` 映射测试）。
- `cargo test -p agent-diva-tools` → 117 passed（含 `with_approval_backend_attaches_guardian_unless_never`）。
- `cargo build -p agent-diva-agent -p agent-diva-manager` → 通过（消费方无破坏）。
- `cargo fmt --all`、`cargo clippy -p agent-diva-sandbox -p agent-diva-tools --all-targets -- -D warnings` → 绿色。

## 覆盖点

| 场景 | 结果 |
|---|---|
| `for_ask` 映射：谨慎→strict、智能→smart、信任→liberal、Never→default | ok |
| 生产接线：信任模式附加 Guardian | ok |
| `Never` 模式保持纯 orchestrator（不附加 Guardian） | ok |
| orchestrator fallback 拆分：信任未知→Skip、谨慎未知→NeedsApproval | ok |
| agent/manager 消费方编译通过 | ok |

## 说明

- 生产行为变化是本 slice 的预期目标（三模式区分生效）；真实桌面 smoke 属后续人工验收项。