# Acceptance — v0.1.0 empty allow_from + QQ reject group

重启网关后：

## A. 空名单可私聊

- [ ] QQ 配置仅有 App ID / Secret，不填允许用户 ID，通道已启用。
- [ ] 用户私聊机器人，Agent 收到消息并回复。
- [ ] 若填写了白名单，名单外用户仍被忽略。

## B. 群聊被拒绝

- [ ] 在 QQ 群 @ 机器人，网关日志出现
      `QQ group/guild message rejected; only C2C private chat is supported`。
- [ ] 该群消息不进入对话 / 不触发回复。
- [ ] 频道（guild）@ 同样不进对话。
