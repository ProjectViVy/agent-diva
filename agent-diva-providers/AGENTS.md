# agent-diva-providers

## OVERVIEW

`agent-diva-providers` holds the LLM provider trait, registry, catalog service, factory, and concrete clients used by the agent loop and report generator. It also contains the Groq-based voice transcription service.

## WHERE TO LOOK

| File / Module | Purpose |
| --- | --- |
| `src/lib.rs` | Primary exports: `LLMProvider`, `DynamicProvider`, `ProviderRegistry`, `ProviderCatalogService`, `build_llm_provider`, `OpenAiCompatibleClient`, `OllamaProvider`. |
| `src/base.rs` | Trait, message/response types, `ToolChoiceMode`, streaming UTF-8 decoder, and model capability helpers. |
| `src/registry.rs` / `src/providers.yaml` | Built-in provider specs and keyword/name lookup. |
| `src/factory.rs` | `build_llm_provider` switches on `ApiType` and wraps the matching client. |
| `src/openai_compatible.rs` | OpenAI-compatible HTTP client, chat, streaming, tool parsing, cache control, retry integration. |
| `src/ollama.rs` | Local Ollama client. |
| `src/anthropic.rs` | Native Anthropic client. |
| `src/catalog.rs` / `src/discovery.rs` | Provider/model catalog views and runtime model discovery. |
| `src/report_narrative.rs` | `LlmReportNarrativeGenerator` for LLM-backed report summaries. |
| `src/transcription.rs` | Groq Whisper `TranscriptionService`. |
| `src/retry.rs` | Shared retry and rate-limit helpers. |
| `tests/` | Integration tests for Ollama tools, streaming, and retry behavior. |
| `examples/` | SiliconFlow chat/TTS/ASR and MiniMax TTS samples. |

## CONVENTIONS

- New providers implement `LLMProvider` and are wired through `build_llm_provider`. Add registry metadata in `providers.yaml`.
- Use `ApiType::Openai` for OpenAI-compatible endpoints and `ApiType::Anthropic` for native Anthropic.
- Model capability checks live in `base.rs`; keep them conservative and add new models explicitly.
- Retry and rate-limit handling stay inside the provider client or `retry.rs`, not callers.
- Transcription uses `TranscriptionService` in `transcription.rs`.
- Report narrative generation uses `LlmReportNarrativeGenerator` in `report_narrative.rs`.

## ANTI-PATTERNS

- Do not rewrite raw model IDs at the provider layer. Native endpoints, e.g. DeepSeek `https://api.deepseek.com/v1`, must send `deepseek-chat`, not `deepseek/deepseek-chat`.
- Only true gateways or aggregators, e.g. OpenRouter, use prefixed model IDs such as `anthropic/claude-sonnet-4`.
- Do not add gateway prefixes inside `resolve_model` or generic request builders.
- Do not rely on tests that only check internal strings. Assert the final outbound JSON `model` field.

## NOTES

- Examples for SiliconFlow and MiniMax live under `examples/`. They are starting points for provider-specific API exploration, not production integration tests.
- `ProviderSpec.gateway_prefix` is descriptive only. Routing code must not derive outbound prefixes from it.
