# 发布说明

- 发布前需要更新并部署包含 `agent-diva-manager` skills API 的后端二进制。
- 若使用桌面 GUI，需要重新构建 `agent-diva-gui` Tauri 应用，使新增的 `get_skills` / `upload_skill` / `delete_skill` 命令生效。
- 本次变更不涉及配置迁移；已有 builtin skills 与 workspace skills 目录结构继续兼容。
- 若运行环境未安装 `just`，可按本次验证记录中的定向命令完成替代校验。

