You are cleaning the dirty working tree of the **agent-diva-selfinprove** repository (branch: `autoresearch/agent-diva-autodream-rhythm-session-history-evid-20260531`). Do NOT pull, push, merge, or rebase. Goal: turn this into a durable research package — README, AutoDream / context-compaction research, shared-memory design, candidate-audit schema, and logs — that can be referenced from mainline later.

## Working directory
`C:\Users\Administrator\Desktop\morediva\agent-diva-selfinprove` (use forward slashes in bash)

## What to do (in this order)

1. **Inventory.** `git status`, `git diff --stat HEAD`. The branch name hints at the topic (autodream rhythm + session history evidence) — confirm the on-disk assets match.

2. **Bucket into 5 themes:**
   - **README + top-level docs** — anything the user needs first to understand what this branch is
   - **AutoDream / rhythm distillation research** — notes, designs, scripts related to the AutoDream feature
   - **Context-compaction research** — notes, comparisons, evidence files
   - **Shared-memory design** — schemas, prototype code, examples
   - **Candidate-audit schema** — YAML/JSON/markdown describing how candidate learnings get audited before promotion
   - Plus `+ logs/evidence` as a 6th bucket for raw session transcripts, log dumps, etc. that probably should NOT be committed as-is

3. **For each theme, output:**
   - File list
   - 1-2 line description
   - Maturity assessment: `prototype` / `design-doc` / `evidence` / `log-dump`
   - Whether it's safe to commit to a docs-only PR, or needs human review first

4. **Tighten loose ends:**
   - Check the top-level README mentions every major subdirectory. If `docs/autodream/`, `docs/context-compaction/`, `docs/shared-memory/`, `docs/candidate-audit/` exist but aren't linked from README, add the links.
   - If there's a `logs/` or `evidence/` directory with >10MB of raw session data, propose `.gitignore` patterns to keep it out of git (or, if it MUST be tracked, propose git-lfs).

5. **DO NOT commit, push, rebase.** All real commits stay with the human.

6. **Final report** ends with `CLEAN_REPORT_SELFINPROVE_DONE`.

## Project rules (MUST FOLLOW)
- This is mostly a research/docs branch, not a feature branch. Light Rust code, lots of markdown.
- Markdown: 1-line H1 title, then H2 sections. No frontmatter unless the file is loaded by a tool that needs it.
- Research notes go under `docs/<topic>/`. ADRs under `docs/adr/NNNN-title.md`.
- Do NOT delete session logs without flagging them first. The user values this evidence.
- Do NOT touch `target/`, `node_modules/`.
- Skip `cargo check` inside your loop.

## Constraints
- `--max-turns`: 15 (smallest of the 4 — 8 files, mostly docs).
- `--allowedTools` limited to: `Read,Edit,Write,Bash(git *),Bash(find:*),Bash(grep:*),Bash(ls:*),Bash(rm:*),Glob,Grep`. (No `cargo` needed unless you find Rust code.) No web, no MCP.
- Single leaf worker.
