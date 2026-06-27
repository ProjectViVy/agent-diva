# Release

- No special deployment step is required beyond shipping the updated workspace binaries.
- Existing config files may be rewritten on first load if they are still at the legacy shape without `config_version`.

# Rollout Notes

- Monitor environments that manage `config.json` externally, because first-run migration now persists `config_version = 2` and the defaulted harness sections.
- If operators diff configs in automation, expect the new root sections to appear after the first successful CLI/config load.
