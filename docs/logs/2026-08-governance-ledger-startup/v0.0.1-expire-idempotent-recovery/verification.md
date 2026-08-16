# 验证记录

| 门 | 命令 | 结果 |
| --- | --- | --- |
| 格式 | `just fmt-check` | PASS |
| Clippy | `just check` | PASS |
| Ledger | `cargo test -p agent-diva-core --lib governance::ledger::tests` | PASS（17/17） |
| 全量 `just test` | 未跑 | 改动限于审批账本重放/分页；由上述聚焦测试覆盖 |

## 观察点

- 对已物化 expire 再调用 `expire()`：事件数不变。
- 手工追加第二条 `expired` 后 `state()` 仍为 Expired。
- `consumed` 后再追加 `expired`：`state()` 失败，`states_page` 只返回邻居。
