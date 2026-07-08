# Verification

- `cargo fmt`
- `cargo test -p agent-diva-core session::manager -- --nocapture`
- `cargo test -p agent-diva-manager handlers::tests -- --nocapture`
- `cargo check -p agent-diva-gui -p agent-diva-manager -p agent-diva-agent -p agent-diva-core`
- `pnpm --dir agent-diva-gui exec vitest run src/components/ConversationSidebar.test.ts`

## Notes

- `pnpm --dir agent-diva-gui exec vitest run src/components/NormalMode.test.ts` is still blocked by the pre-existing `/miku.svg` asset-resolution failure already tracked in `TODOLIST.md`.
