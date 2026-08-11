# Verification

## 命令与结果

- `cargo test -p agent-diva-sandbox` → 127 passed（含新增
  `append_amendment_shared_persists_and_updates_in_memory`）。
- `cargo test -p agent-diva-tools` → 117 passed。
- `cargo build -p agent-diva-agent -p agent-diva-manager` → 通过。
- `cargo fmt --all`、`cargo clippy -p agent-diva-sandbox -p agent-diva-tools --all-targets -- -D warnings` → 绿色。

## 覆盖点

| 场景 | 结果 |
|---|---|
| `append_amendment_shared` 更新内存策略（后续 evaluate 命中 Allow） | ok |
| 学到的规则落盘到指定位文件 | ok |
| 重复规则被拒绝 | ok |
| `policy()` 改返回 `Arc<Policy>` 无外部调用方破坏 | ok |
| agent/manager 编译通过 | ok |

## 说明

- 自动学习端到端（trusted 放行→create_rule→落盘→后续 known-safe）在 exec_policy +
  orchestrator 层验证；真实桌面 smoke 属后续人工验收项。