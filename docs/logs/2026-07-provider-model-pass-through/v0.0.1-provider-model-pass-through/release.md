# Release

## Method

Ship as a focused provider-layer runtime fix. No configuration migration is required because the behavior preserves existing `model` strings exactly as configured.

## Operator Notes

- Native OpenAI-compatible endpoints should now receive raw model IDs when configured that way.
- Gateway deployments that require prefixed IDs must store the prefixed ID explicitly in configuration.
- Provider registry `litellm_prefix` and `skip_prefixes` remain in the schema but are no longer used to rewrite outbound model IDs.
