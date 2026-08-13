# Summary

- Fixed runtime model discovery classification so providers are no longer limited to a small hardcoded whitelist.
- Any provider with `api_type = openai` and an effective `api_base` now attempts OpenAI-compatible `GET /models`.
- For providers that still fall back to bundled static models because runtime discovery is unsupported, the fallback response is now quiet instead of emitting a misleading warning card.
- Added a defensive guard in `LiteLLMClient::resolve_model` so malformed URL-like `litellm_prefix` metadata does not turn into broken prefixed model ids.
