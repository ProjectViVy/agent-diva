# Acceptance

## Functional Checks

1. Start the updated agent runtime and complete an analysis-oriented conversation about repository structure, documentation, or implementation planning.
2. Confirm the turn still behaves normally and `MEMORY.md` injection behavior is unchanged.
3. Verify a new file appears under `workspace/memory/diary/rational/YYYY-MM-DD.md`.
4. Confirm the new diary entry contains:
   - title
   - summary
   - timestamp
   - observations
   - confirmed items
   - next steps
   - source references when file paths were mentioned
5. Run a casual or purely social turn and verify no new rational diary entry is written.

## Non-Regression Checks

1. Existing session persistence still works.
2. Existing consolidation still updates `MEMORY.md` / `HISTORY.md`.
3. Prompt construction still does not inject diary files into the model context.
