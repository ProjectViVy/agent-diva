# Verification

## Method

- Read `docs/dev/genericagent/newedge/architecture.md`.
- Read `docs/dev/genericagent/newedge/ui-design.md`.
- Updated `docs/dev/genericagent/README.md` links and reading order.
- Ran `git diff --check` for the touched documentation paths.

## Result

- NewEdge documents are indexed from the README.
- No whitespace errors were reported by `git diff --check`.

## Not Run

- `just fmt-check`, `just check`, and `just test` were not run because this change only updates Markdown documentation and does not touch Rust, configuration, or executable behavior.
