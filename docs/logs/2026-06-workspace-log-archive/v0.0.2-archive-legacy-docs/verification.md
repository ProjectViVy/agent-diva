# Archive Legacy Docs Verification

## Commands

- `git diff --diff-filter=D --name-only -z -- docs ':!docs/resources'`
- `git restore --pathspec-from-file=/tmp/agent-diva-deleted-docs.nul --pathspec-file-nul`
- `find docs/past/legacy-docs -type f | wc -l`
- `git status --short --untracked-files=all`

## Result

- 196 historical documentation files were restored and moved into `docs/past/legacy-docs/`.
- Non-document deleted paths remain separate for independent handling.
- No Rust or GUI validation was run because this change only reorganizes documentation/history files.
