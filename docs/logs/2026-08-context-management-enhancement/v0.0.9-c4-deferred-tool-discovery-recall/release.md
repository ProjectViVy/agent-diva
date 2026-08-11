# Release

本迭代不执行发布或推送。变更以三个聚焦 Conventional Commit 保存在当前 `agent-diva-pro` 分支，待用户评审后再决定合并/发布。

发布前应确认：workspace validation、CLI help smoke 以及必要的真实 MCP source hot-reload 验收均完成；当前自动化覆盖 source unregister 后的稳定 unavailable 行为。
