# 重要代码示例

`CommandApprovalRequest { approval_id, command, cwd, reason, available_decisions }` 是跨层契约。持久规则只接受已解析的命令 token 前缀，并拒绝 PowerShell、shell、解释器和提权前缀。
