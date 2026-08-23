# v0.2.0 WorkspaceContext 工作区合同：总结

> 状态：骨架（Wave C1 完成后补全）

## 交付范围

- `agent-diva-core` 新增 `WorkspaceContext` / `WorkspaceSource`（ExplicitCli /
  Configured / ProcessCwd / LegacyDefault）与单一解析函数 `resolve_workspace`。
- 优先级：CLI 显式 > config 明确保存值 > 进程 CWD；立即绝对化/canonicalize，
  路径不存在报清晰错误。
- 旧默认 `~/.agent-diva/workspace` 标记 LegacyDefault，一次性 warn + doctor 提示，
  不静默切换。
- CLI/Gateway/Manager workspace 推导统一走解析函数；禁止全局 `set_current_dir`。
- 模板写入收缩：`ensure_workspace_templates` 仅在 onboard / config refresh /
  workspace create 显式路径调用，启动链不再自动写模板。
