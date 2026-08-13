You are cleaning the dirty working tree of the **agent-diva-pro** repository (branch: `feature/context-compaction`). Do NOT pull, push, merge, or rebase. Goal: untangle the mixed work (implementation + ADRs + research notes + pet/UI requirements) and report how to split it into 3 clean lines: `context-compaction`, `thinking`, and `pet-ui`.

## Working directory
`C:\Users\Administrator\Desktop\morediva\agent-diva-pro` (use forward slashes in bash)

## What to do (in this order)

1. **Inventory first.** Run `git status`, `git diff --stat HEAD`, and `git log main..HEAD --oneline` (or whatever the base branch is — check `git merge-base HEAD origin/main 2>/dev/null` or fall back to `git log --oneline -10`). Report high-level shape: file count, dirs, recent commits.

2. **Cluster the dirty files into exactly 3 buckets:**
   - **context-compaction** — anything implementing the compaction algorithm, state, or tests
   - **thinking** — anything implementing the reasoning/thinking capability, config, GUI integration
   - **pet-ui** — anything touching the embedded pet view, fullscreen, multimodal paste
   - Use `+ untracked` as a 4th bucket for files that don't fit any of the 3 (call this out separately — these are usually docs/ADRs/notes that should be committed to a docs-only commit or extracted to `docs/research/`)

3. **Untracked asset audit.** Many untracked files in `docs/research/`, `docs/adr/`, `docs/epics/`, `docs/prd/` are legitimate research artifacts. Bucket them by topic. Flag any that are clearly stale (>6 months old AND no recent references) or duplicated across the 3 buckets.

4. **DO NOT commit, rebase, or push.** Real commits stay with the human.

5. **For each of the 3 buckets, output:**
   - File list (use `git diff --name-only HEAD` filtered, or list untracked files in that bucket)
   - Why these files belong together (1-2 lines)
   - Entanglement warnings: which files touch 2+ buckets and would need to be split
   - Suggested commit message prefix (e.g. `feat(compaction):`, `feat(thinking):`, `feat(pet):`)

6. **Final report** ends with `CLEAN_REPORT_PRO_DONE`.

## Project rules (MUST FOLLOW)
- Rust workspace (cargo). 4-space indent, rustfmt defaults.
- Vue 3 + TypeScript for GUI. Composition API + `<script setup lang="ts">`.
- All UI strings go through i18n locale files — never hardcode.
- Do NOT add new dependencies. Flag instead.
- Do NOT touch `target/`, `Cargo.lock` diffs, `node_modules/`.
- Skip `cargo check` inside your loop. Only run once at the very end with `--message-format=short`.
- **UI protection rule (CRITICAL):** if you see mass deletion of Vue components or CSS files, STOP and flag — do not "fix" or restore. Report it as a potential merge regression and let the human verify.
- All ADRs go under `docs/adr/NNNN-title.md` (4-digit, kebab-case). Research notes go under `docs/research/<topic>/`. Do not invent new top-level doc dirs.

## Constraints
- `--max-turns`: 20. Batch reads.
- `--allowedTools` limited to: `Read,Edit,Write,Bash(git *),Bash(find:*),Bash(grep:*),Bash(ls:*),Bash(cargo *),Bash(rm:*),Glob,Grep`. No web, no MCP.
- Single leaf worker — do not delegate.
