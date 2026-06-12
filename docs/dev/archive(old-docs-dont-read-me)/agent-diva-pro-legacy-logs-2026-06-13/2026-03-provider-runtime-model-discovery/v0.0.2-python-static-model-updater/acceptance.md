# Acceptance

1. Run `python scripts/update_provider_models.py --providers openrouter --dry-run` and confirm the tool prints fetched and final model counts without writing `providers.yaml`.
2. Run `python scripts/update_provider_models.py --providers openrouter` and confirm the target provider's `models:` block in `agent-diva-providers/src/providers.yaml` is updated.
3. Run `python scripts/update_provider_models.py --providers openrouter openai --sort --keep-existing` and confirm both providers are processed and the resulting YAML remains structurally valid.
