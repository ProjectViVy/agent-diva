# Token 统计链路修复

Manager 现在从 `.agent-diva/token_ledger.jsonl` 聚合 Token 用量，并向 GUI 提供总量、分组、趋势、会话、模型和全量实时统计接口。GUI API 已改为直接消费 Tauri 命令返回的 DTO。
