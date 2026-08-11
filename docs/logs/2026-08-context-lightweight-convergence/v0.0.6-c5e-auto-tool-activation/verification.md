# Verification

- `cargo check -p agent-diva-tooling -p agent-diva-tools -p agent-diva-agent --lib`：通过。
- `cargo test -p agent-diva-tooling registry --lib`：24 passed。
- `cargo test -p agent-diva-tools tool_discovery --lib`：1 passed。
- `cargo test -p agent-diva-agent deferred_tool_search_auto_activates_for_the_next_same_turn_call --lib`：通过。
- `cargo test -p agent-diva-agent --lib`：389 passed。
- deletion-proof `rg`：产品源码不再包含旧 discovery state、mount tool、not-discovered/not-mounted
  protocol、旧 metadata key 或 legacy result fallback。

覆盖 search → next provider call → direct execution、搜索替换、8 项上限、registry rebuild
保留同回合 activation、来源下线不可执行和新 AgentLoop 不恢复旧 metadata。
