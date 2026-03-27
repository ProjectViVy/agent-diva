# Acceptance

## 验收步骤

1. 准备一个包含 `memory/MEMORY.md` 与 `memory/diary/rational/*.md` 的 workspace。
2. 发送类似“之前我们对 memory 拆分做了什么结论？”、“最近关于 provider 的结论是什么？”、“当前项目进度到哪了？”的问题。
3. 确认 agent 能直接给出基于记忆的回答，而不是完全依赖模型猜测。
4. 确认这类问题的当前 turn 中存在 auto-recalled memory 上下文，但 session history 中没有新增 recall 摘要消息。
5. 发送纯闲聊或纯执行类请求，例如“你好”或“请执行 cargo test”，确认不会自动触发 recall policy。

## 预期结果

- 历史/进度/偏好/承诺/结论类问题会自动消费记忆。
- recall 结果为压缩摘要，不会把 diary 原文或整份 `MEMORY.md` 全量塞进 prompt。
- memory 工具仍可被模型继续主动调用，用于 recall 不足时的细化查询。
