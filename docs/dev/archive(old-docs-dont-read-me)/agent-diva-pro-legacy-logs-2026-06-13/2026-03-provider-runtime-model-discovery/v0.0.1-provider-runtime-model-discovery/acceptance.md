# Acceptance

1. Run `agent-diva provider models --provider openai --json` against a configured OpenAI-compatible endpoint and confirm the response contains `source: "runtime"` plus live model IDs.
2. Run `agent-diva provider models --provider anthropic --static-fallback --json` and confirm the response contains `source: "static_fallback"` plus bundled Anthropic models.
3. Open the GUI Providers settings page, select a provider, click `Refresh online models`, and confirm the model list updates plus shows `Live catalog` or `Static fallback`.
4. Call `GET /api/providers/<provider>/models` and confirm the JSON payload includes `provider`, `source`, `runtime_supported`, `models`, `warnings`, and `error`.
