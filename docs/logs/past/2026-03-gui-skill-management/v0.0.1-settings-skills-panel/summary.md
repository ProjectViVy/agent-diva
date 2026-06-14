# 迭代摘要

- 目标：在 GUI 设置板块中新增独立的“技能”入口，支持上传 ZIP 技能包、删除工作区技能，并展示全部可见 skills 与 active 状态。
- 后端：`agent-diva-manager` 新增 `/api/skills` 列表、ZIP 上传、删除接口，并增加 `skill_service` 处理 workspace/builtin skill 合并、active 判定、ZIP 解压和 builtin 删除保护。
- Tauri：新增 `get_skills`、`upload_skill`、`delete_skill` 命令，统一代理到本地 manager HTTP API。
- 前端：设置首页新增“技能”卡片，`SettingsView` 新增独立 `skills` 页面，`SkillManagementCard` 负责列表、上传、删除和浏览器预览降级提示。
- 文案：补充英文文案与中文 patched i18n 文案，覆盖技能页标题、状态标签、上传限制和错误提示。

