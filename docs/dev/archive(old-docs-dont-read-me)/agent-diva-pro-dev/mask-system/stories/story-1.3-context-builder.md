# Story 1.3: Runtime Mask Activation via ContextBuilder

**Epic:** Epic 1 — Mask Management & User-Facing Lifecycle
**Status:** ready-for-dev
**Priority:** P0
**Depends on:** 1.1, 1.2

## Story

As a DiVA user,
I want activating a mask to inject its prompt into the system prompt,
So that the agent behaves according to the mask's role.

## Acceptance Criteria

- [ ] AC1: Mask prompt is appended after SOUL/IDENTITY prompts (方案 A)
- [ ] AC2: Default mask injects no additional prompt
- [ ] AC3: Mask model override takes precedence over global default

## Tasks

- [ ] Create `agent-diva-agent/src/mask/mask_prompt_composer.rs` — MaskPromptComposer
- [ ] Modify `agent-diva-agent/src/context.rs` — add mask parameter to build_system_prompt()
- [ ] Implement prompt hierarchy: system base → 松本 core → mask
- [ ] Add unit tests for prompt composition

## Dev Notes

- Architecture A-3: MaskPromptComposer as intermediate layer
- ContextBuilder calls MaskPromptComposer::compose()
- Keep existing behavior when no mask is active

## File List

- `agent-diva-agent/src/mask/mask_prompt_composer.rs` (new)
- `agent-diva-agent/src/context.rs` (modify)