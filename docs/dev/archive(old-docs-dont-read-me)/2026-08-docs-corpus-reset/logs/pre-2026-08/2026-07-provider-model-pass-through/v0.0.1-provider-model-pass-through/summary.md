# Provider Model Pass-Through

## Summary

Runtime model resolution no longer auto-adds LiteLLM-style `provider/model` prefixes. `LiteLLMClient::resolve_model()` now treats `model` as an opaque configuration string and returns it unchanged for every provider/base combination.

## Changes

- Removed runtime branches that inferred native-provider vs gateway behavior from `provider_name`, `api_base`, registry `litellm_prefix`, or `skip_prefixes`.
- Kept provider registry metadata intact for catalog/default/display use.
- Updated provider tests so DeepSeek, OpenRouter, StepFun, explicit prefixed model IDs, and unknown custom providers all assert model pass-through.

## Impact

Existing saved `model` values are preserved exactly. Configurations that intentionally use prefixed model IDs still send those prefixed strings, while raw native model IDs such as `deepseek-chat` and `step-3.7-flash` are no longer rewritten.
