# Approval single-channel and expiry synchronization

GUI 仅启动统一 `start_approval_stream`、仅消费 `approval-event`；删除 legacy Tauri
command、invoke 注册、emit 与 capability 描述。`ApprovalCenterCard` 在 TTL 归零时
只发起一次详情刷新，不在前端伪造 `expired`；刷新失败继续保持 0 秒和批准禁用。
