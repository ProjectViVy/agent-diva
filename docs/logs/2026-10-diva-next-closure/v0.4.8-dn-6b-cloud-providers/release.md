# v0.4.8 release — DN-6B

One commit on `feat/dn-closure-wave1`:

- `feat(speech): add bounded cloud media and cancellation`

New files: `crates/diva-speech/src/{wav,registry,providers,service}.rs`,
`crates/diva-speech/tests/speech_contract.rs`,
`scripts/ci/fixtures/legacy-calls/{speech-browser-fetch,pet-restore}.ts`.

Modified: `crates/diva-speech/{Cargo.toml,src/lib.rs,src/credentials.rs}`,
`agent-diva-gui/src-tauri/{Cargo.lock,src/lib.rs,src/speech/mod.rs,
src/speech/commands.rs}`, `scripts/ci/check_legacy_frontend_calls.mjs`,
`scripts/ci/check_vivy_backend_boundary.py`.

No push, no PR/merge, no release. Baseline pins unchanged
(DIVA f5866a0 / VIVY 1db8b55 / Laputa dc6066e / INOFY 71e2c9b).
LOCK.md remains held by this session.
