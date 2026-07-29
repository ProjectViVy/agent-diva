# GUI Lucide Migration Acceptance

- Install dependencies with `pnpm install`.
- Run `pnpm test` and confirm the full suite passes.
- Run `pnpm build` and confirm type checking and bundling complete.
- Start `pnpm preview` and confirm the application entry page loads.
- Confirm repository GUI source and tests contain no `lucide-vue-next` imports.
- Spot-check chat, settings, planning, evolution, and Diva Pet views to confirm icons remain visible and interactive controls retain their behavior.
