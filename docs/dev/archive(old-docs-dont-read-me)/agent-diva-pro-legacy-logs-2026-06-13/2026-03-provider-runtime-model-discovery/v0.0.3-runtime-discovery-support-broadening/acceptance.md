# Acceptance

1. Open the GUI provider settings page and select an OpenAI-compatible provider with a valid `api_base`; confirm it now attempts runtime discovery instead of immediately reporting `unsupported`.
2. Select a provider that still lacks runtime discovery support and confirm the UI falls back to bundled models without showing the previous misleading warning text.
3. Run `agent-diva provider models --provider <provider> --json` for an OpenAI-compatible provider and confirm the response reports `source = "runtime"` when the endpoint is reachable.
