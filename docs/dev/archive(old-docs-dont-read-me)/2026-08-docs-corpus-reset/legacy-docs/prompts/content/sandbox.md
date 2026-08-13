You are cleaning the dirty working tree of the **agent-diva-sandbox** repository (branch: `agent-diva-with-sandbox`). Do NOT pull, push, merge, or rebase. Goal: bucket the 45+ modified files into 4 migration slices (PR #1~#4) so the user can land them as a stacked migration to the main branch.

## Working directory
`C:\Users\Administrator\Desktop\morediva\agent-diva-sandbox` (use forward slashes in bash)

## What to do (in this order)

1. **Read the migration plan first.** Look for any of: `MIGRATION_PLAN.md`, `docs/migration/`, `sandbox-migration*`, or TODOs at the repo root. If found, use that as the authoritative slice breakdown. If not, INFER the slices from the file content (the typical split is: new crate scaffolding → manager wiring → GUI integration → docs/scripts).

2. **Inventory.** `git status`, `git diff --stat HEAD`, and the migration plan. Report the file count vs the plan's expected slice sizes — flag any major mismatches.

3. **Bucket files into 4 slices.** Default slice names (override if the migration plan says otherwise):
   - **Slice 1 — Crate scaffolding:** new `crates/*` directory trees, root `Cargo.toml` workspace edits that add new members, any `crates/<new>/Cargo.toml`
   - **Slice 2 — Manager wiring:** changes to `crates/manager/**`, runtime-control changes that integrate with the manager
   - **Slice 3 — GUI integration:** Vue components, locales, route changes that surface the sandbox feature
   - **Slice 4 — Docs + scripts:** markdown, shell scripts, CI yml, anything in `scripts/` or `docs/`
   - Add a 5th bucket `+ cross-cutting` for files that legitimately touch 2+ slices (e.g. shared error types, root `lib.rs` exports). These are the merge headaches — flag them loudly.

4. **For each slice, output:**
   - File list (relative paths)
   - Approx LOC
   - Why it belongs in this slice
   - Cross-cutting dependencies on other slices
   - Suggested PR title: `sandbox: <slice name>`

5. **Final report** ends with `CLEAN_REPORT_SANDBOX_DONE`.

## Project rules (MUST FOLLOW)
- Rust workspace (cargo). 4-space indent, rustfmt defaults.
- Vue 3 + TypeScript for GUI. Composition API + `<script setup lang="ts">`.
- All UI strings go through i18n locale files.
- Do NOT add new dependencies. Flag instead.
- Do NOT touch `target/`, `Cargo.lock` diffs, `node_modules/`.
- Skip `cargo check` inside your loop. Only run at the very end with `--message-format=short`.
- **UI protection rule (CRITICAL):** any mass deletion of Vue components or CSS is a merge regression — STOP and flag, do not "fix".
- This repo's whole point is the sandbox feature — preserve the SecurityPolicy / permission logic verbatim. Do not "refactor" or "simplify" it during cleanup.

## Constraints
- `--max-turns`: 25 (this is the largest of the 4 — 45 files, 4 slices).
- `--allowedTools` limited to: `Read,Edit,Write,Bash(git *),Bash(find:*),Bash(grep:*),Bash(ls:*),Bash(cargo *),Bash(rm:*),Glob,Grep`. No web, no MCP.
- Single leaf worker.
