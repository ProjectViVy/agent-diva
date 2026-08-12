# Acceptance

待用户配合真实桌面执行：

1. cautious 模式对高风险 shell 生成且只生成一个 Drawer 审批项，允许后恢复执行。
2. smart 模式安全操作直执行，高风险操作进入审批。
3. trusted 模式成功学习允许规则，重复安全操作不再询问；危险操作仍受 Guardian 限制。
4. 重启 GUI 后 permission mode 保持；审批到期最终显示服务端 `expired`。

本轮未执行上述真机步骤，不能标记为通过。
