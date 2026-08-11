# Verification

## 命令与结果

- `cargo test -p agent-diva-core rejection_circuit` → 6 passed（阈值命中、窗口滑过恢复、
  reset 清除、clone 共享窗口、config merge、默认大阈值）。
- `cargo test -p agent-diva-core config` → 73 passed（含新增 `test_config_merge_copies_rejection_circuit_fields`、
  `test_default_rejection_circuit_is_a_large_dead_loop_safety_valve`）。
- `cargo test -p agent-diva-agent rejection_circuit` →
  `test_rejection_circuit_reads_config_values` ok（从 security.json 读取 window/threshold 并构造）。
- `cargo clippy -p agent-diva-core -p agent-diva-agent --all-targets -- -D warnings` → 绿色。

## 覆盖点

| 场景 | 结果 |
|---|---|
| 阈值命中触发（< 阈值不触发，== 阈值触发） | ok |
| 窗口滑过（1s 窗口，sleep 2s）后恢复 | ok |
| reset 强制清除触发态 | ok |
| Clone 共享同一计数窗口 | ok |
| 配置 merge 复制新键 | ok |
| 默认值为大阈值死循环安全阀 | ok |
| agent 接线从 security.json 读取并构造 breaker | ok |

## 说明

死循环熔断的 e2e 触发路径（provider 连续失败 N 次后拒绝新一轮）依赖外部 provider
故障注入，未在本 slice 内做真实 provider smoke（用户已将其列为后续桌面/灰度验收项）。