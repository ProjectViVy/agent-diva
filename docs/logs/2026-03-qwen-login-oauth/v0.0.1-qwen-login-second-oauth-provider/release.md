# Release

- 本轮未执行正式发布。
- 原因：这是 workspace 内部开发迭代，且 `agent-diva-gui --no-run` 在当前环境被磁盘容量阻塞，尚未达到完整交付门槛。
- 若后续释放磁盘空间并补齐 GUI 编译验证，可按常规 Rust workspace 流程继续执行 `just fmt-check`、`just check`、`just test` 与 GUI smoke。
