# Issue #32 P4.3 verification

## Regression evidence

Before the fix, focused Vitest observed two `onVivyEvent` calls for concurrent
subscribers and an unhandled rejection after a shared installation failure.
The tests cover one shared installation, per-handler delivery/unsubscribe,
close while installation is pending, close after installation, post-close
subscription, consistent rejection, and clean retry.

## Verification outcomes

- `./node_modules/.bin/vitest run src/api/vivy/transport.test.ts
  src/api/vivy/client.test.ts` — passed, 15 tests across 2 files.
- `./node_modules/.bin/vue-tsc --noEmit` — passed.
- `./node_modules/.bin/vite build` — passed; Vite emitted its existing warning
  that several minified chunks exceed 500 kB.
- Vitest reported no unhandled rejection on the corrected run.

The repository `pnpm --dir agent-diva-gui test ...` entrypoint attempted to
remove/reconcile `node_modules` and aborted because the process has no TTY.
No dependency files were changed; verification used the already installed
package-local binaries directly.
