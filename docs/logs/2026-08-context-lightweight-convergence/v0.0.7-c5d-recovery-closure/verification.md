# Verification

- `cargo check -p agent-diva-agent --lib`：通过。
- `cargo test -p agent-diva-agent context_assembly --lib`：19 passed。
- `cargo test -p agent-diva-agent context_budget --lib`：21 passed。
- `cargo test -p agent-diva-agent compaction --lib`：28 passed。
- C5e 合并后的 agent 全部 lib 测试：389 passed；C5c/C5e artifact 与 deferred tool 回归均在其中。
- deletion-proof：agent/core/tooling/tools 产品源码不含旧 discovery state、mount 协议、旧
  metadata key、legacy count-cap 或 provider 结果 fallback。

边界覆盖：checkpoint token 计入总预算、Recall Drop 优先级、StablePrefix/Checkpoint/ActiveTail
区域计量、完成/未完成 tool group、provider 失败不产生 pending durable update、reactive 失败
保留完整 history，以及 session checkpoint 的 durable index 下界。
