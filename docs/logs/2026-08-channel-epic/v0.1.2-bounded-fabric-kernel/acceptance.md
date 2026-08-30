# CHANNEL-EPIC C2a 验收

## 自动验收

1. 运行 `cargo test -p agent-diva-core --test channel_fabric_tck`，确认 8 项全部通过。
2. 确认 ingress 第 257 项在 deadline 后返回 Busy，且未被 consumer 接受。
3. 确认 control 在普通 ingress 与 transient 饱和时仍优先消费。
4. 确认 durable 满载 producer 等待容量，transient 同 key 合并并在置换时发出 Gap。
5. 确认同 session 提交顺序不变、不同 session 可同时进入 sink。
6. 确认 shutdown 后拒绝新项并排空已接受项。

## 产品验收

本批没有用户可见运行时变化，不执行真实频道或 GUI smoke。产品切换验收留到 C5/C6。
