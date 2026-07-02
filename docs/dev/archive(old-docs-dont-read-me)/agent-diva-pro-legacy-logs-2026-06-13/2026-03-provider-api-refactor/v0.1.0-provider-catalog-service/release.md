# Release

- 发布方式：未执行正式发布。
- 原因：本次为本地实现与验证迭代，未包含版本发布、打包或部署请求。

# Rollout Notes

- 若后续需要发布，建议先解除 Windows 下 `target\debug\agent-diva.exe` 的文件锁，再补跑一次完整 `just test`。
