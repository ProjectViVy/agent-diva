# Acceptance

- 新建 root session 的 `/api/sessions` 摘要含稳定 workspace identity、channel、root kind
  和自身 root key。
- 通过显式 child seam 创建 branch/subagent/ephemeral 时，摘要含真实 root、parent 与可选
  label；不允许把 root 当 child，也不允许 parent 不存在。
- 旧 JSONL 仍可列出，显示为独立 legacy root；不会根据 session key 猜测分支或父子关系。
- API 仍以当前运行时 workspace 的 session authority 为边界，不建立跨 workspace 全局历史索引。
