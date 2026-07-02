# Summary

- Added `agent-diva provider` command group with `list`, `status`, `set`, and placeholder `login`.
- Moved CLI default-provider model selection to registry metadata by adding `default_model` to `ProviderSpec` / `providers.yaml`.
- Extended direct CLI chat surface with `--markdown/--no-markdown`, `--logs/--no-logs`, explicit `--session`, and new lightweight `chat` subcommand.
- Updated command index documentation for `provider`, `chat`, and `/new-command`.
