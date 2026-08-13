# Acceptance

## User-Facing Checks

- Selecting an OpenAI-compatible provider builds `OpenAiCompatibleClient`.
- Selecting Anthropic builds `AnthropicClient` and sends native `/v1/messages` payloads.
- Provider model IDs are passed through exactly as configured.
- `providers.yaml` uses `gateway_prefix` in active entries.
- Older provider metadata containing `litellm_prefix` still deserializes.
- GUI provider connection tests use the same provider factory as CLI and manager runtime.

## Follow-Up

- Run a real Anthropic smoke when `ANTHROPIC_API_KEY` is available.
- Normalize pre-existing `agent-diva-e2e` formatting drift so `cargo fmt --check` can be restored as a clean workspace gate.
