# Summary — v0.1.0 empty allow_from + QQ reject group

用户确认：空 `allow_from` 必须与 GUI「留空表示不限制」一致；QQ 不接群聊，群消息拒绝、不进对话。

## 变更

- `BaseChannel::new` 恢复 `deny_by_default = false`。空名单允许所有发送者。
  需要拒绝默认的通道继续用 `with_default_policy(..., true)`（Matrix 未改）。
- QQ 只转发 `C2C_MESSAGE_CREATE`。`GROUP_AT_MESSAGE_CREATE` /
  `AT_MESSAGE_CREATE` / `GROUP_MESSAGE_CREATE` 记日志、去重后丢弃，不发 inbound。
- 不实现群发送 API。

## 影响范围

- `agent-diva-channels` 的 `base.rs` / `qq.rs`。
- 飞书 / Discord / 钉钉 / Email 等走 `BaseChannel::new` 的通道，空名单同样恢复为不限制。
