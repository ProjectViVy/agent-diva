# v0.3.0 AGENTS.md 注入 MVP：验收步骤

> 状态：骨架（Wave C2+C3 完成后补全）

1. workspace 根放置 `AGENTS.md`，发起会话，确认系统提示只注入一次受预算包裹的项目指令。
2. 删除 `AGENTS.md` 并 `invalidate`，确认不再注入且无报错。
3. 让模型调用 Shell 且 `working_dir` 指向 workspace 外，确认被拒绝并返回根目录约束说明。
4. 运行 `agent-diva status`（或 doctor），确认输出 AGENTS 注入状态与 digest，无正文回显。
