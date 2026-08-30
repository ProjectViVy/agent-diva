# CHANNEL-EPIC C2b 验收

## 自动验收

1. 运行 `cargo test -p agent-diva-channels --test channel_adapter_runtime_tck`，确认 11 项通过。
2. 确认 unsupported capability 在 adapter execute 前失败。
3. 确认第 130 个 adapter egress 请求返回 Busy，其他 adapter 仍可独立投递。
4. 确认 Retry-After 优先于本地退避，且无幂等保证的命令不自动重试。
5. 确认文本先分片，失败分片阻止后续发送并返回 `partial_delivery` receipt。
6. 确认 listener exit/error/panic 不终止进程，第三次连续失败进入 Down，stop 打断 backoff。
7. 确认 fake smoke 从 Fabric ingress 得到 `delivered` receipt。

## 产品验收

本批没有真实平台和 GUI 行为变化，不执行真实频道 smoke。C5 的每频道 TCK/真实样板与 C6
原子切换承担产品验收。
