# Summary

- Added `scripts/update_provider_models.py` as a manual developer tool for refreshing static provider `models` entries in `agent-diva-providers/src/providers.yaml`.
- The script fetches live model IDs from selected OpenAI-compatible providers and replaces or merges the YAML `models:` block for the chosen providers.
- Added dry-run, provider selection, sorting, and merge controls so maintainers can update bundled catalogs intentionally instead of mutating them at runtime.
