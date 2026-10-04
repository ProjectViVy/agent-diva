# Verification — v0.4.14 DN-8C

- JSON validity: tauri.conf.json parses; version fields consistent across
  package.json / Cargo.toml (shell + diva-speech) / Cargo.lock.
- Existing checks re-run green on the branch: vitest 67/551, cargo
  bridge+speech 25, both CI gates clean.
- `windows-shell-check` authored but not yet executed — requires push to
  GitHub windows-latest runner; real-artifact FFI tests stay env-gated
  until a Windows-staged `vivy-shared.dll` exists.
