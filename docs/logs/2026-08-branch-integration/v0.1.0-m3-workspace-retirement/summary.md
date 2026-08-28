# M3 HITL and Workspace Branch Retirement

## Summary

将适用于当前 `dev` 的 M3 GUI 修正合入主线，并退休已完成且内容重复的旧分支。

## Changes

- 将 `m3-hitl` 的审批卡片 TTL 预警和英文文案合入 `dev`，提交为 `13500dfe`。
- `App.vue` 中的旧审批流启动调用在当前 `dev` 已存在，因此没有重复提交。
- 保留 `dev` 现有审批数据流和冲突解决后的字段集合，没有引入旧分支的过时代码。
- 删除已合入且 worktree 干净的 `agent-diva-m3-hitl` worktree 与 `feat/m3-hitl-closure` 分支。
- 核对 `feat/workspace-agents-impl`：WorkspaceContext、AGENTS.md 合同、Shell 工作区边界、`GET /api/workspace`、测试和迭代日志均已等价存在于 `dev`。
- 未合并该分支中过时的 `AGENTS.md`、`README.md`、`TODOLIST.md`，随后删除 `feat/workspace-agents-impl` 分支。

## Impact

产品代码只增加审批 TTL 临近过期提示；workspace 与 M3 的已完成实现继续由当前 `dev` 作为唯一来源。
