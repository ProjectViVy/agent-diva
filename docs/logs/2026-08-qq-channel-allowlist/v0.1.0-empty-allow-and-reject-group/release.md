# Release — v0.1.0 empty allow_from + QQ reject group

需重启网关（`agent-diva` / `just diva-gate`）后生效。GUI 无需重编。

配置里 QQ 只填 `app_id` / `secret` 并 `enabled: true`、不写 `allow_from`，即可收 C2C 私聊。群 `@` 会被拒绝，不会进 Agent。
