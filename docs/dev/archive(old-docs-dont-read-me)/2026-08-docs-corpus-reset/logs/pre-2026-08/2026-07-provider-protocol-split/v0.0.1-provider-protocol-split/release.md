# Release

## Method

- Land as one focused provider-routing change.
- No migration command is required.
- Existing serialized provider specs using `litellm_prefix` remain readable through the serde alias and map to `gateway_prefix`.

## Operator Notes

- Anthropic native configs should use:
  - `provider = anthropic`
  - `api_base = https://api.anthropic.com`
  - raw Claude model IDs such as `claude-sonnet-4-5`
- OpenAI-compatible gateways should keep explicit gateway model IDs in config/catalog.
- The provider layer does not add or strip `provider/model` prefixes at runtime.
