# P3-15 Provider API Error Structured

## Summary

- Added `ProviderApiError` with status, provider, model, code, message, error type, retry-after, and request id fields.
- Changed `ProviderError::ApiError` to carry a boxed `ProviderApiError` so the error enum remains clippy-clean.
- Added `ProviderError::api_message(...)` for message-only API errors.
- Updated provider error detection helpers and existing provider error call sites.
- Structured LiteLLM/OpenAI-compatible non-2xx responses while preserving non-JSON bodies as messages.

## Impact

- Provider API errors can now retain routing and response metadata for logging, UI, and retry policy decisions.
- Existing message-only tests and mocks can continue to construct API errors through the helper.
