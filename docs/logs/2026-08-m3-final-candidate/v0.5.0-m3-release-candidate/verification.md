# Verification

## Candidate identity

- Git: `2fbb07d3e5579c0b68338fa1f8243cf5e815739c`
- Release EXE: `target/release/agent-diva-gui.exe`
- Size: `176827392` bytes
- SHA-256: `77887F513E46CAF79F6A98D31034024B20DEF66EE2DD434DECB24FE21333B609`

## Required automated matrix

- `cargo clippy -p agent-diva-core --all-targets -- -D warnings` — passed.
- `cargo clippy -p agent-diva-manager --all-targets -- -D warnings` — passed.
- `cargo clippy -p agent-diva-gui --all-targets -- -D warnings` — passed.
- `cargo test -p agent-diva-core` — passed.
- `cargo test -p agent-diva-sandbox` — passed.
- `cargo test -p agent-diva-laputa` — passed.
- `cargo test -p agent-diva-manager` — passed.
- `cargo test -p agent-diva-cli` — passed.
- `cargo test -p agent-diva-gui` — 55 library and 11 integration tests passed.
- `pnpm --dir agent-diva-gui test` — 58 files, 446 tests passed.
- `pnpm --dir agent-diva-gui build` — passed; existing chunk-size advisory only.
- `just fmt-check`, `just check`, `just test` — passed.

Only the existing `imap-proto v0.10.2` future-incompatibility advisory remains;
the two previously recorded unused test-fixture warnings are gone.

## Windows release access evidence

The complete Tauri release rebuilt successfully. ACL permits execution, the file
has no Zone.Identifier, no Agent Diva AppLocker/Code Integrity/Defender event was
found, and source/copies had identical SHA-256. Direct, workspace-copy and
`C:\tmp`-copy starts all returned Windows OS error 5. The artifact is unsigned;
no trust store, Defender/EDR, ACL, administrator or system-policy change was made.
This is the first hard gate in final human acceptance.
