# Verification

## 命令与结果

- `cargo test -p agent-diva-sandbox` → 125 passed（含新增 4 个三模式契约测试 +
  1 个 cautious-asks-on-skip）。
- `cargo fmt --all` → 通过。
- `cargo clippy -p agent-diva-sandbox --all-targets -- -D warnings` → 绿色。

## 覆盖点

| 场景 | 结果 |
|---|---|
| 谨慎（strict, OnRequest）：只读/危险/未知全部询问 | ok |
| 谨慎在策略 Skip 时仍询问（不再 Defer） | ok |
| 智能（OnFailure）：只读自动放行、危险/未知询问 | ok |
| 信任（liberal, UnlessTrusted）：未知自动放行 + 自动学习（create_rule=true）、危险询问 | ok |
| `Never` 在 Skip 时仍 Defer（行为不变） | ok |
| clippy 严格模式无告警 | ok |

## 说明

- 三模式契约测试在 Guardian 层验证；生产行为不变（Guardian 未接线）。
- 故障注入/真实 provider smoke 不适用于本 slice（纯逻辑判定）。