# Release

本迭代只交付隔离分支上的共享代码、测试和开发说明，不发布二进制、不启用 Manager
native registry、不切换生产频道、不合入 `dev`、不 push。

交付提交：

1. `da7e0410 docs: correct channel adapter development contract`
2. `87e7ff20 feat: add shared channel adapter services seam`

下一接收动作：以 `87e7ff20` 为共同基线创建 Telegram/Discord/Feishu 与 DingTalk/Email/QQ
的独立 worktree，并严格遵守一频道一 agent 的文件 ownership。QQ 仍需先完成官方 intents、
group send 和 media 证据。
