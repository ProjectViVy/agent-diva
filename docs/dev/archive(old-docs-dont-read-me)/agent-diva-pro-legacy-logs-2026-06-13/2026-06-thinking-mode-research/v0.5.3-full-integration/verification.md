# Verification

## Commands

```bash
cargo fmt                                   # formatting applied
cargo check -p agent-diva-core              # clean
cargo check -p agent-diva-agent             # clean
cargo check -p agent-diva-cli               # clean
cargo test -p agent-diva-providers          # 60 passed
cargo test -p agent-diva-agent              # 72/73 passed
```

## Smoke Test

```bash
# CLI help should include /thinking
cargo run -p agent-diva-cli -- chat --help
```

## Known Issues

- `skills::tests::test_default_builtin_dir_loads_skills` fails (pre-existing, unrelated to thinking mode)
- GUI build not verified (requires `npm run build` in agent-diva-gui)
