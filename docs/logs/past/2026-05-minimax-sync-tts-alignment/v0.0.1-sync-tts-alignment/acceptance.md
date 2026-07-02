# 验收步骤

## Example

1. 配置 `MINIMAX_API_KEY`。
2. 运行 `cargo run -p agent-diva-providers --example minimax_sync_tts -- "你好"`。
3. 确认输出包含连接成功、分块接收、`is_final` 和音频保存信息。

## GUI

1. 打开 `Diva Pet` 设置面板，选择 `MiniMax`。
2. 确认默认 `Base URL` 为 `https://api.minimaxi.com`。
3. 确认默认 `Model` 为 `speech-2.8-hd`。
4. 确认默认 `Voice ID` 为 `male-qn-qingse`。
5. 触发一次测试播报或正常回复播报，确认仍走 MiniMax TTS 合成链路。

## 通过标准

- 仓库中不再保留旧示例名称的运行说明。
- Example 与 GUI 对 MiniMax 的默认模型一致。
- 同步合成示例可生成有效音频文件。
