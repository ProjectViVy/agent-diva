# Phase 1: Model Capabilities Reasoning Whitelist

## Summary

在 `model_capabilities_for_model()` 中添加了 reasoning 模型白名单，
使 `ModelCapabilities.reasoning` 字段从硬编码 `false` 变为按模型动态判断。

## Changes

- `agent-diva-providers/src/base.rs`:
  - 新增 `is_known_reasoning_model()` 私有函数，覆盖 20+ reasoning 模型
  - `model_capabilities_for_model()` 调用该函数填充 `reasoning` 字段
  - 新增 `supports_reasoning_model()` 公开 API
  - 新增 `reasoning_capabilities_are_conservative` 测试用例
- `agent-diva-providers/src/lib.rs`:
  - 导出 `supports_reasoning_model`

## Validation

- `cargo check -p agent-diva-providers` — clean
- `cargo fmt` — applied
- `cargo test -p agent-diva-providers` — 60 tests passed (56 unit + 4 integration)

## Covered Models

| Provider | Models |
|----------|--------|
| DeepSeek | deepseek-chat, deepseek-reasoner, deepseek-r1 |
| Anthropic | claude-3-opus, claude-3.5-sonnet, claude-3.7-sonnet, claude-3.5-haiku, claude-sonnet-4, claude-opus-4 |
| OpenAI | o1, o1-mini, o3-mini, o1-pro |
| Gemini | gemini-2.0-flash-thinking, gemini-2.5-pro, gemini-2.5-flash |
| Qwen | qwen-max, qwen-plus, qwq-32b |
| Doubao | doubao-pro-32k, doubao-lite-32k |
