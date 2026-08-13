# Story 6.2 Acceptance

## Acceptance Steps

- Trigger apply failure injection and confirm the section content is restored, rollback/changelog staging files are cleaned, and the proposal becomes `needs_attention`.
- Trigger migration failure after section commit and confirm prior section and state files are restored.
- Submit an unresolved-conflict proposal and confirm apply returns `conflict_unresolved` behavior and marks the proposal `needs_attention`.
- Submit a schema-incompatible proposal through the manager route and confirm the HTTP error body contains `code: "schema_incompatible"`.
- Poll Laputa error events and confirm recoverable failures emit `status: "needs_attention"` and a concrete `error_type`.

## Result

All Story 6.2 acceptance criteria are covered by targeted automated tests.
