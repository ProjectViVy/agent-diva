# 验收步骤

1. 打开 GUI，进入“设置”首页，确认出现独立的“技能”卡片入口。
2. 点击“技能”，确认进入独立技能页面，而不是“通用”页面内嵌卡片。
3. 页面加载后确认可看到全部可见 skills，且每项显示来源（Builtin/Workspace）与状态（Active/Available/Unavailable）。
4. 选择一个包含 `SKILL.md` 的 ZIP 技能包上传，确认列表自动刷新并出现新技能。
5. 找到 builtin skill，确认删除按钮为禁用态，且不可删除。
6. 找到 workspace skill，执行删除，确认删除成功且列表刷新。
7. 若删除的是覆盖 builtin 的 workspace skill，确认同名 builtin skill 重新出现在列表中。

