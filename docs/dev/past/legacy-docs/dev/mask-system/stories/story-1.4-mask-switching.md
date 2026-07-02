# Story 1.4: Implement Mask Switching with Context Compression

**Epic:** Epic 1 — Mask Management & User-Facing Lifecycle
**Status:** ready-for-dev
**Priority:** P0
**Depends on:** 1.2, 1.3

## Story

As a DiVA user,
I want to switch masks seamlessly with clean context,
So that the agent doesn't carry stale behavior from a previous mask.

## Acceptance Criteria

- [ ] AC1: `/mask wear <name>` compresses context, injects switch message, injects mask prompt
- [ ] AC2: `/mask off` returns to default mask with same sequence
- [ ] AC3: Switch message notifies LLM of mask change

## Tasks

- [ ] Implement mask switching logic in MaskRegistry
- [ ] Integrate with context compression (existing `/compress` mechanism)
- [ ] Add switch notification injection
- [ ] Add CLI command handling for `/mask wear`, `/mask off`, `/mask status`

## Dev Notes

- Context compress → inject switch message → inject mask prompt
- Preserve session continuity (don't reset conversation)

## File List

- `agent-diva-agent/src/mask/mask_registry.rs` (modify)
- `agent-diva-cli/src/chat_commands.rs` (modify)