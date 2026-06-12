# Acceptance

1. Open `docs/dev/archive/nano/nano-decoupling-preparation-plan.md`.
2. Confirm the document states that:
   - `agent-diva-cli` is the formal product line.
   - `agent-diva-manager` remains the default runtime/control-plane dependency for CLI.
   - `agent-diva-nano` is treated as a template/starter rather than the formal release line.
3. Confirm the document separates coupling into compile-time, runtime, and release-time categories.
4. Confirm the document recommends delaying workspace extraction of `nano` until the main product closure is stable.
