# Verification

## Focused coverage

- `pnpm --dir agent-diva-gui test -- src/components/ApprovalCenterCard.test.ts src/components/ApprovalCenterDrawer.test.ts src/components/ChatView.test.ts`
  - 3 files, 11 tests passed.
  - Covers: no floating trigger when closed, risk sort, domain filter, card allow/deny/evidence/outcome-unknown, history toggle overlay, icon under history + pending badge + open emit.

## Deferred

- Full `just fmt-check` / `just check` / `just test` not required for this frontend-only layout pass; no Rust sources changed.
- Desktop visual smoke (theme contrast, drawer keyboard loop in a live window) remains for the next manual GUI smoke.
