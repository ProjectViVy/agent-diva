# 错误处理与边界条件

## 工具输入

空清单、超过 20 项、空步骤、超长字段、未知状态和多个 `in_progress` 均返回 `ToolError::InvalidArguments`，不会发布 checklist update。

## 流式边界

- checklist 事件早到：更新仍在运行的工具行。
- checklist 事件晚到：更新当前轮已完成的 `update_plan` 工具行。
- 未找到工具行：卡片插入流式 assistant 之前。
- 未找到 assistant：若 final 有内容则创建完成消息，并始终清理请求状态。
- 连续用户轮次：搜索遇到最近 user 消息即停止，禁止覆盖旧清单。

## 取消与错误

既有 stop/error 路径保持独立；本修复不吞掉 provider 或工具错误。事件解析失败继续由现有日志记录，不将不可信内容拼接为命令或 HTML。
