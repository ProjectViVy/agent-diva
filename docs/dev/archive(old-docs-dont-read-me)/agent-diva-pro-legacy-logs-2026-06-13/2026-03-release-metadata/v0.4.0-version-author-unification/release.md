# Release

## 发布方式

- 本次仅完成版本号与作者元数据统一，未执行正式发布。
- 如需正式发布 `0.4.0`，应先解决现有格式检查失败与 `target/debug/agent-diva.exe` 占用问题，再执行既定发布流程。

## 发布前建议

- 清理或停止占用 `target/debug/agent-diva.exe` 的进程。
- 处理现有未格式化文件后重新执行 `just fmt-check`、`just check`、`just test`。
- 确认 CLI `--version`、GUI About、安装包文件名均显示 `0.4.0`。
