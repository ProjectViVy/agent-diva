# Verification

## Focused coverage

- `pnpm --dir agent-diva-gui test -- src/components/ApprovalCenterCard.test.ts src/components/ApprovalCenterDrawer.test.ts src/components/ChatView.test.ts src/api/approvals.test.ts` — 12 passed.
- `cargo test -p agent-diva-manager handlers::approvals::tests` — 9 passed.
- `pnpm --dir agent-diva-gui build` — passed.
- `cargo check -p agent-diva-gui` — passed.

The focused tests prove exact badge count, risk ordering, domain filtering, explicit grant emission, non-color status/TTL/scope text, high-risk Memory evidence blocking, outcome-unknown action disabling, reconnect event deduplication, old-version rejection, and suppression of duplicate legacy Command cards.

## Full affected suites

- `pnpm --dir agent-diva-gui test` — 58 files, 446 tests passed.
- `cargo test -p agent-diva-manager` — 98 passed, 1 ignored.
- `cargo test -p agent-diva-gui` — 55 library tests and 11 integration tests passed.

## Delivery gates

- `just fmt-check` — passed.
- `just check` — passed; only the existing `imap-proto v0.10.2` future-incompatibility advisory was emitted.
- `just test` — passed; the two previously recorded unused-variable warnings in test fixtures remain unchanged.

The Vite large-chunk advisory remains unchanged.

Per the frozen Goal cadence, visual desktop observation is not performed at this intermediate stage. Badge placement, drawer presentation, keyboard traversal, inline consistency, reconnect, stale, and edit-and-approve are included in the single final Epic/M3 manual smoke.
