# Verification

- `cargo check -p agent-diva-core -p agent-diva-tooling -p agent-diva-agent --lib`：通过。
- `cargo test -p agent-diva-core tool_artifact --lib`：6 passed。
- `cargo test -p agent-diva-tooling registry --lib`：23 passed。
- `cargo test -p agent-diva-agent tool_results --lib`：3 passed。
- `cargo test -p agent-diva-agent --lib`：后续 C5e 合并后 389 passed，证明 C5c 接口在完整 agent 流程中保持稳定。

重点覆盖：inline 边界、artifact ref 稳定性、容量失败显式错误、microcompact 幂等性、当前
工具组保护和 registry 不截断完整清洗结果。
