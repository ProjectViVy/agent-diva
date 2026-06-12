# MiniMax 同步合成链路收敛总结

## 本次变更

- 将 `agent-diva-providers` 的 MiniMax 示例从误导性的 `async` 命名和 HTTP `POST /v1/t2a_v2` 调整为 WebSocket 同步合成实现。
- 将 GUI MiniMax 默认模型统一为 `speech-2.8-hd`：
  - Tauri `pet_minimax_synthesize`
  - 设置页默认值
  - 前端 TTS provider 默认配置
  - 相关前端测试断言
- 清理仓库中的旧示例命名和运行说明，统一为 `minimax_sync_tts`。

## 影响范围

- `agent-diva-providers`：MiniMax example 与依赖。
- `agent-diva-gui`：MiniMax 默认 model 与测试。
- `docs/logs`：旧示例记录和本次交付记录。
