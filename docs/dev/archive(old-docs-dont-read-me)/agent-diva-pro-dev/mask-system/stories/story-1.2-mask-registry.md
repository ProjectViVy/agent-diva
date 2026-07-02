# Story 1.2: Build Mask Registry from Workspace Directory

**Epic:** Epic 1 — Mask Management & User-Facing Lifecycle
**Status:** ready-for-dev
**Priority:** P0
**Depends on:** 1.1

## Story

As a DiVA user,
I want the system to scan `workspace/masks/` and build a registry of available masks,
So that I can see and use all masks I've created.

## Acceptance Criteria

- [ ] AC1: Registry scans `workspace/masks/` and parses all `.md` files with valid frontmatter
- [ ] AC2: Supports nested subdirectories (e.g., `coding/rust-coder.md`)
- [ ] AC3: Invalid mask files are skipped with warning log
- [ ] AC4: Registry returns list of all valid masks with metadata

## Tasks

- [ ] Create `agent-diva-agent/src/mask/mask_registry.rs` — MaskRegistry struct
- [ ] Implement directory scanning with `walkdir` or `std::fs`
- [ ] Implement mask caching (in-memory HashMap)
- [ ] Add `get()`, `list()`, `reload()` methods
- [ ] Add unit tests

## Dev Notes

- Architecture A-1: File system scan on startup
- Handle missing directory gracefully (return empty list)
- Use `tracing::warn!` for parse failures

## File List

- `agent-diva-agent/src/mask/mask_registry.rs` (new)
- `agent-diva-agent/src/mask/mod.rs` (modify)