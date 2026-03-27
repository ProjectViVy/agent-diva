# Summary

- Restored backward-compatible `name` output in CLI provider JSON entries while keeping the current `provider` field.
- Updated the CLI integration test to assert the canonical `provider` field and explicitly verify the compatibility alias.
- Scope is limited to provider status/list JSON serialization in `agent-diva-cli`.
