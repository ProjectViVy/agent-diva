# C1d Cache Observability Summary

## 完成内容

- 新增 provider prompt-cache profile 与 `ToolDefinitionSet` CORE 边界，过滤后仍能稳定
  选择 CORE 段末缓存锚点。
- OpenAI-compatible 与原生 Anthropic 仅标记第一条 stable system；后续 compaction/system
  块不带 control。禁用缓存的 provider 请求保持不变。
- AgentLoop 每次实际模型调用记录 SHA-256 system/tools/per-tool hash、break reason、
  provider/model/policy/TTL、stable prefix token 估算及 cache read/create 趋势。
- Anthropic cache usage 透传至 `LLMResponse.usage` 与 JSONL token ledger 的可选字段；
  旧 ledger 记录继续可读。

## 边界

- Observer 窗口仅驻留进程内；未增加 dashboard、指标后端或 TTL 配置。
- C1c 外部 L1 apply / Skills reload 通知仍按既有 TODOLIST 独立延期。
