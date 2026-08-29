# CHANNEL-EPIC C1 完成摘要

## 完成内容

- 在 `schemas/neuro-link/v1/protocol.schema.json` 建立 Neuro-Link v1 的唯一 wire schema，
  固定 JSON-RPC 2.0、精确版本、请求/通知/响应方法集合、typed envelope、内容块、receipt、
  cursor、error 和扩展命名空间规则。
- 在 `agent-diva-core::channel` 建立 transport-neutral Rust contracts，包括
  `ChannelEnvelopeV1`、`ChannelAddress`、`Correlation`、`ContentPart`、payload、receipt、
  JSON-RPC 请求/通知/响应及 session/turn/state 参数。
- 在 GUI 建立与 schema 对齐的 TypeScript contract facade；当前 UI 不提前接入，避免在 C4
  之前形成第二条实时链路或兼容层。
- 建立 Rust/TypeScript 共用正反 fixture 和 TCK，覆盖 hello、service list、session/open、
  turn/start、turn/cancel、event/ack、state/resume、stream、错误、未知字段、版本不匹配、
  旧 pipe、伪造身份、扩展名和 malformed content。
- 以 characterization 固定旧 MessageBus FIFO/单消费者/关闭错误、allowlist、loopback guard
  和稳定错误文本；以固定代码容量冻结 C2 的五条 lane profile。

## 容量基线

| Lane | 固定容量 |
| --- | ---: |
| control | 64 |
| ingress | 256 |
| durable event | 512 |
| transient event | 128 |
| adapter egress | 128 |

容量是代码常量，不是用户配置，也没有复制 Octos 的 4096。

## 边界

C1 只交付合同、fixture、表征测试和基准，不实现 Gateway、Fabric Kernel、Adapter、Journal、
GUI 实时迁移或旧树删除；这些工作分别留给 C2～C6。没有加入兼容 DTO、shim、双轨路径或身份
伪装字段。
