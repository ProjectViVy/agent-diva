# WS-04：会话层级持久化与 API 合同

本阶段为 workspace 内 session authority 增加稳定的 lineage 表征：

- `SessionInfo` 增加 `workspace_id`、`channel`、`kind`、`root_session_key`、
  `parent_session_key`、`branch_label` 和 `legacy`。
- 新 root session 由 `SessionManager` 写入 workspace identity、首个 channel、root kind
  和自身 root key；显式 child 创建要求存在 parent，并写入真实 root/parent 关系。
- 旧 JSONL 不被扫描器改写；缺少 lineage metadata 时只读投影为独立 `legacy=true` root，
  channel 仅从 key 的首个分隔符解析，不推断 parent/branch。
- Manager 的现有 `/api/sessions` DTO 自动携带这些字段，仍然只扫描当前运行时 workspace
  的 `sessions/` 目录。
