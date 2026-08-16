# 验证记录

| 门 | 命令 | 结果 |
| --- | --- | --- |
| 格式 | `just fmt-check` | PASS |
| Clippy | `just check` | PASS |
| Core lib | `cargo test -p agent-diva-core --lib` | PASS（697/697） |
| Core 集成 | `cargo test -p agent-diva-core --test security_integration` | PASS（5/5） |
| Agent | `cargo test -p agent-diva-agent --lib test_process_direct_keeps_original_pii_shaped_text` | PASS |
| 全量 `just test` | 未跑 | 改动限于安全检查与工具结果改写；由上述聚焦测试覆盖 |

## 观察点

- 默认 `redact_pii` 不改写邮箱、手机、记录 id。
- `sanitize_tool_output` 对含 id / 邮箱 / 手机 / 卡号的 JSON 原样返回。
- `check_security("Contact me at test@example.com")` 为 `Allow`。
- artifact 回读保留 `sk-…` 原文。
