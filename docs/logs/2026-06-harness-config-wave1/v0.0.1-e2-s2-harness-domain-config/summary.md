# Summary

- Added root config coverage for `security`, `presence`, `heartbeat`, `audit`, `pii`, and `injection` in `agent-diva-core/src/config/schema.rs`, with default values and `config_version`-aware schema support.
- Wired harness-domain validation and hot-reload diff extraction so invalid thresholds fail at the config boundary and presence/PII/injection changes are detectable.
- Connected runtime bootstrap to use configured security, presence, and heartbeat values, and added CLI regression coverage for explicit harness config plus legacy-config migration.

# Impact

- CLI config surfaces now expose harness-domain sections as first-class root config instead of silently falling back to disconnected defaults.
- Legacy config files without the new harness sections still load through the CLI, auto-migrate to `config_version = 2`, and persist stable defaulted values for the new domains.
