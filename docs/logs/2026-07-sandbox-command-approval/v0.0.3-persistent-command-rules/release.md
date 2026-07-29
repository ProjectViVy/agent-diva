# Release

No remote push or deployment was performed. The feature is available in the next desktop build.

Existing `prefix_rules` TOML remains readable. Missing metadata is derived with a stable legacy ID and defaults to enabled. Invalid TOML fails Manager startup instead of silently dropping policy. New writes use a temporary file followed by a replace-and-flush operation, and update memory only after persistence succeeds.

Rollback consists of reverting the two feature commits and this documentation commit. Operators may retain or remove `~/.agent-diva/execpolicy.toml`; older builds ignore the new workflow unless explicitly wired to that file.
