# Acceptance: v0.4.10-irc-sasl-clippy-fix

## 验收步骤

1. 在仓库根目录运行 `just check`。
2. 确认输出中不再出现 `agent-diva-channels/src/irc.rs` 的 `collapsible_match` 报错。
3. 运行 `cargo test -p agent-diva-channels test_encode_sasl_plain`。
4. 确认测试通过，且无新增 IRC SASL 相关失败。

## 验收标准

- CI 阻塞错误消失。
- 相关测试保持通过。
- IRC SASL 分支行为与修复前一致。
