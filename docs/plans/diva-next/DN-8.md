# DN-8 — Clean package and final owner acceptance

Current scope: [DN-C1](p0-design.md); status/dependencies: [index.md](index.md).
Consumes scoped DN-M closure, refreshed DN-P artifact evidence and packaged
OBS evidence. **DN-7 is cancelled and is not a predecessor.** This revises the
old full-parity/import premise; it does not authorize release or claim checks.

## Delivery boundary

The product contains the existing Tauri/vivy-bridge shell, sealed VIVY library
and narrow native media services. No retired Rust business runtime, second
Agent store/executor or hidden Manager fallback is bundled. New native speech
preferences/credentials/assets are an explicitly admitted device boundary.

## Engineering preparation

- Pin source, recipe, generated header/ABI, Generation and runtime hashes;
  rebuild through the existing pack/inspect path and invalidate old evidence.
- Run scoped GUI/build, bridge/native command, domain/runtime, AST/dependency /
  package checks. Reuse existing gates; amend them narrowly for speech.
- Prepare clean-checkout build and fresh-home installation/start/key setup,
  permissions/device prompts, diagnostics and bounded Quit instructions.
- Provide real-model chat/tool/approval/cancel/window-reopen scenarios;
  cognition human edits/model projection/capture/review/restart; console
  logs/trajectory/usage gaps; microphone/STT/both TTS/interrupt/replay exclusion.
- Prepare Windows/amd64 DLL/header/FFI/bundle checks. Run available native
  checks and identify unavailable ones; do not block independent implementation
  waiting for the owner's final desktop test.
- Resolve governance-document conflicts only with explicit instruction-file
  rewrite authorization. Record deferred residual surfaces honestly.

## Final acceptance

The owner performs installed-product acceptance after engineering delivery.
Provide expected effects and failure cases with source/artifact pins; mark
untested platform/cloud behavior pending. Do not claim acceptance from merged
PRs, mocks or prior-pin Linux evidence. No historical importer scenario,
automatic publication, tag or merge belongs to this task.
