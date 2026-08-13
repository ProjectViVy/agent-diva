You are cleaning the dirty working tree of the **agent-diva** repository (branch: `main`). Do NOT pull, do NOT push, do NOT merge anything. Goal: produce a clean, reviewable grouping of the uncommitted changes so the user can later decide how to split them into commits or PRs.

## Working directory
`C:\Users\Administrator\Desktop\morediva\agent-diva` (use forward slashes in bash)

## What to do (in this order)

1. **Inventory first.** Run `git status` and `git diff --stat HEAD`. Report the high-level shape of the dirty tree in plain text: how many files, which top-level directories, rough theme (runtime / GUI / docs / etc.). Do NOT list every file yet.

2. **Group by theme.** Read the diff (or `git diff HEAD` piped to your analysis) and bucket the changes into 3-7 themes. For each theme, list:
   - Theme name (e.g. "runtime-control tweaks", "GUI/locales", "docs cleanup")
   - File count
   - 1-2 line description
   - Whether the change is self-contained or entangled with another theme

3. **Tighten loose ends.** Look specifically for these "scattered" items and call them out:
   - Broken links in `docs/dev/README.md` (run `grep -rn "\[\^\]\|](./docs/" docs/ 2>/dev/null | head` and check)
   - `decisions.md` updates that need a new ADR
   - Untracked logs, scratch files, or `*_log.md` artifacts in `docs/`
   - `plan-todo-ui-scope-extract.md` and any sibling planning docs that should be consolidated
   - `.todolist.md` or similar local TODO files (NOT `TODOLIST.md` in the repo, that's tracked)

4. **DO NOT commit, rebase, or push.** Your job ends at the analysis + the on-disk cleanup that the analysis surfaces (e.g. removing scratch files, fixing obvious broken links, putting stray docs into the right subdir). All real commits stay with the human.

5. **Output a final report.** A markdown block summarizing:
   - Theme groupings (table: theme | files | description | entangled)
   - "Quick wins" — 3-5 items you cleaned up automatically (e.g. removed 3 log files, fixed 2 broken links)
   - "Needs human decision" — anything that should be a follow-up card on the kanban, with a one-line spec

## Project rules (MUST FOLLOW)
- This is a Rust workspace (cargo). 4-space indent, rustfmt defaults.
- All UI strings go through the i18n locale system — never hardcode English in Vue/components.
- Do NOT add new dependencies. If you think one is needed, flag it in the report instead of adding.
- Do NOT touch `target/`, `Cargo.lock` diffs, or anything in `node_modules/`.
- Do NOT run `cargo build` or `cargo check` from inside your loop — they eat turns. Only run them at the very end as a sanity check, with `--message-format=short` to limit output.
- If you find a Vue/CSS file that looks like it might be a UI regression (mass deletion of components, gutted CSS), STOP and flag it — git merge silently destroys these. Do not "fix" it yourself.

## Constraints
- `--max-turns` budget: 20. Be efficient — read in batches, don't re-read the same file.
- Use `--allowedTools` limited to `Read,Edit,Write,Bash(git *),Bash(find:*),Bash(grep:*),Bash(ls:*),Bash(cargo *),Bash(rm:*),Glob,Grep`. No web tools, no network, no MCP.
- Do not delegate to other agents. You are a single leaf worker.
- Final report must end with the line `CLEAN_REPORT_MAIN_DONE` so the orchestrator knows you finished.
