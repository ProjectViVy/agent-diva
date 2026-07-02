# CLEAN REPORT — agent-diva-sandbox

**Branch**: `agent-diva-with-sandbox`
**Date**: 2026-06-06
**Claude session**: 1e4b4fce-641b-408b-9b24-5652272fea3b
**Cost**: $0.79 / 66 turns

## Inventory
- 34 modified + 57 untracked = 91 dirty files
- Net: +1402 / -59
- New crate LOC: 7,259 (agent-diva-sandbox)

## 4-Stacked-PR Migration Plan
| PR | Theme | Files | LOC | Depends on |
|---|---|---|---|---|
| #1 | Crate scaffolding + core types | 22 | ~7,400 | — |
| #2 | Manager/agent/tools wiring | 17 | ~870 | #1 (hard) |
| #3 | GUI integration (Vue + i18n + Tauri) | 9 | ~677 | #2 (hard) |
| #4 | Docs + scripts + changelog | 46 | ~5,600 | independent |

## 5 piggy-backed unrelated changes (MUST separate)
1. `agent-diva-providers/src/lib.rs` — adds `pub mod ollama;`
2. `agent-diva-providers/Cargo.toml` — adds `uuid` dep
3. `agent-diva-manager/src/file_service.rs` — `file_name` → `filename` rename
4. `agent-diva-tools/src/attachment.rs` — `FileMetadata` import path
5. `agent-diva-tools/src/filesystem.rs` — unused `_temp_dir` rename

→ Cherry-pick into `chore: misc fixes` BEFORE sandbox PRs

## 1 misplaced feature
- `docs/logs/2026-04-session-compaction/` (4 files) is a SEPARATE feature (session context compaction), not sandbox. Exclude from sandbox PRs.

## Cross-cutting risks
- `agent-diva-core/src/config/schema.rs` defines `SandboxConfig` etc. used by BOTH core (serde) AND sandbox crate (runtime) — intentional duplication via `from_core_config()` bridge, but creates maintenance coupling
- `agent-diva-tools/src/shell.rs` (+341) imports from both core AND sandbox — primary integration point, breaks if core schema changes
- `Cargo.lock` must be regenerated after each slice merges

## UI protection
- ✅ Additive only in Vue (SandboxSettings.vue new, SettingsView/Dashboard additive). No mass deletions.
- ✅ SecurityPolicy preserved verbatim

## Suggested merge order
PR#1 → PR#2 → PR#3 (strict chain). PR#4 can parallel any of them.

CLEAN_REPORT_SANDBOX_DONE
