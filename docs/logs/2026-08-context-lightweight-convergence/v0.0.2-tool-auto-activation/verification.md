# Verification

- 确认 TODOLIST 新增独立 C5e 切片并给出顺序、容量、回收和恢复边界。
- 确认技术规格区分 Installed、Authorized、Active 与 Executable 责任。
- 确认自动激活不绕过 builtin、MCP、mask、plan、read-only 或 approval。
- 确认 clean-break 明确删除 `mount_tool`、旧 discovery metadata 和兼容路径。
- 执行 `git diff --check`；无生产代码变化，不运行 Rust 构建或测试。
