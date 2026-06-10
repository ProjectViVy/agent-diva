# Acceptance

## Acceptance Steps

- Trigger an upstream provider non-2xx response through the LiteLLM-compatible client.
- Confirm the resulting `ProviderError::ApiError` includes status, model, provider when known, retry-after, request id, and OpenAI-compatible error fields when provided.
- Confirm message-only API errors still work through `ProviderError::api_message(...)`.
- Confirm context overflow and vision unsupported detection still work for API errors.
