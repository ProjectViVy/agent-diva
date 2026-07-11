# Verification

## Commands

```text
cargo test -p agent-diva-core revision_hash -- --nocapture
cargo check -p agent-diva-gui
cd agent-diva-gui; npx vue-tsc --noEmit
```

## Results

| Check | Result |
| --- | --- |
| `revision_hash_is_stable_and_content_sensitive` | pass |
| `revision_hash_survives_display_trim_when_renormalized` | pass |
| `cargo check -p agent-diva-gui` | pass |
| `vue-tsc --noEmit` | pass |

## Deferred

- Manual GUI smoke: Plan generate → 批准并开始执行 (needs running gateway + LLM).
