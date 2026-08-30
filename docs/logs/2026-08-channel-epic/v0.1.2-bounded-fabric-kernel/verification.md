# CHANNEL-EPIC C2a 验证记录

## 定向验证

- `cargo test -p agent-diva-core --test channel_fabric_tck`：8 passed。
- `cargo clippy -p agent-diva-core --lib -- -D warnings`：通过。
- `cargo check -p agent-diva-core --bench channel_capacity`：通过；capacity benchmark 已改为
  调用真实 control/ingress/durable/transient Fabric API。

Fabric TCK 覆盖 ingress 满载 Busy、control 优先、durable 背压、transient coalesce/Gap、
request fence、drain shutdown、同 session FIFO、跨 session 并行以及生产 Fabric 模块无
`unbounded_channel`。

## 阶段门禁

- `just fmt-check`：通过。
- `just check`：通过；仅有既有 `imap-proto` future-incompatibility 提示。
- `just test`：通过；workspace unit、integration 和 doc tests 全绿，既有 ignored tests 保持原状。

全量测试仍显示既有测试代码 unused-variable warning，不影响 `just check` 或测试结果，本批未改动
无关代码。
