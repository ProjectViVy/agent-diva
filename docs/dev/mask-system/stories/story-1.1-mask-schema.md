# Story 1.1: Define Mask Schema and Default Mask

**Epic:** Epic 1 — Mask Management & User-Facing Lifecycle
**Status:** ready-for-dev
**Priority:** P0

## Story

As a DiVA user,
I want the system to define a clear mask file format and ship a built-in default mask,
So that the mask system has a stable foundation and I always have a fallback.

## Acceptance Criteria

- [ ] AC1: Mask schema supports Markdown + YAML frontmatter with fields: name, icon, description, model, subagent_defaults, tool_limits
- [ ] AC2: Built-in default mask "我就是我" is available when no masks exist
- [ ] AC3: Default mask has no extra prompt, no tool limits, no model override

## Tasks

- [ ] Create `agent-diva-core/src/config/schema.rs` — add MaskConfig, ToolLimits structs
- [ ] Create `agent-diva-agent/src/mask/error.rs` — MaskError enum
- [ ] Create `agent-diva-agent/src/mask/mask_file.rs` — MaskFile struct + YAML parsing
- [ ] Add unit tests for MaskFile parsing
- [ ] Add default mask constant

## Dev Notes

- Architecture A-2: Code goes in `agent-diva-agent/src/mask/`
- Use `serde_yaml` for frontmatter parsing
- Follow project naming: `snake_case` files, `PascalCase` types
- MaskError uses `thiserror`

## File List

- `agent-diva-core/src/config/schema.rs` (modify)
- `agent-diva-agent/src/mask/mod.rs` (new)
- `agent-diva-agent/src/mask/error.rs` (new)
- `agent-diva-agent/src/mask/mask_file.rs` (new)