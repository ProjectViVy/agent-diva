# Provider Protocol Split Summary

## Changed

- Renamed the OpenAI-compatible chat-completions client from `LiteLLMClient` to `OpenAiCompatibleClient` and moved the module to `openai_compatible.rs`.
- Added a native `AnthropicClient` for `/v1/messages` with message, tool, tool-result, usage, reasoning, and basic SSE conversion.
- Added `build_llm_provider(...)` factory routing `ApiType::Openai` to OpenAI-compatible and `ApiType::Anthropic` to the native Anthropic client.
- Rewired CLI, manager runtime/hot-swap, and GUI provider test construction through the shared factory.
- Replaced active provider registry `litellm_prefix` keys with `gateway_prefix` while preserving serde alias compatibility for old configs.
- Updated Anthropic catalog defaults to `https://api.anthropic.com` and raw `claude-sonnet-4-5`.
- Disabled proxies for local/plain HTTP provider bases to keep local gateways and mock discovery requests from being routed through system proxy layers.

## Impact

- Native provider model IDs remain opaque pass-through strings.
- Anthropic no longer uses an OpenAI-compatible request body when selected by provider metadata.
- OpenRouter/AiHubMix and other gateways remain OpenAI-compatible; gateway model IDs are user/catalog-provided and are not rewritten at runtime.
