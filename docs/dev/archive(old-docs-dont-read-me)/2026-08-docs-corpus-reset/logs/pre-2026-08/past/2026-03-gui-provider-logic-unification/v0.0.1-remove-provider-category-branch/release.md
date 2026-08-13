# Release

- 本次为本地实现与验证迭代，未执行正式发布、打包或部署。
- 若需要交付：
  - 重新启动使用中的 `agent-diva` / `agent-diva-gui` 本地进程，确保载入新的 provider 逻辑；
  - 在没有占用 `target\debug\agent-diva.exe` 的环境中补跑一次完整 `just test`。
