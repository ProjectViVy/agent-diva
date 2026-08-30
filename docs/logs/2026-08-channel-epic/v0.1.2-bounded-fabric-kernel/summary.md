# CHANNEL-EPIC C2a 完成摘要

## 完成内容

- 在 `agent-diva-core::channel` 建立 bounded Fabric Kernel，实际使用 C1 冻结的 control 64、
  ingress 256、durable 512、transient 128 容量。
- ingress 支持 caller deadline/cancellation、满载 typed Busy 和 retry hint；control 使用独立容量
  并优先于普通 ingress。
- durable 满载时施加背压而不静默丢弃；transient 同 key 合并最新值，容量置换产生 Gap。
- request fence 丢弃取消后的晚到 transient；shutdown 停止新 admission 并排空已接受项。
- `FabricIngressScheduler` 以有界任务预算保证同 session 顺序、跨 session 并行，且不改变
  Session Admission 对 queued/started/rejected 的权威。

## 边界

C2a 未接入或修改旧 `MessageBus`、`ChannelManager`、`ChannelHandler`，未建立兼容桥或生产
切换；Projection Journal、Gateway 和 adapter egress 分别留给 C3/C2b。
