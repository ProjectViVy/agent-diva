# Story 2.2 Proposal Inbox Acceptance

## Acceptance Steps

1. Open the Evolution workspace and stay on the Inbox tab.
2. Confirm each proposal row shows type, state, risk, target, source, evidence count, age, unread/deferred markers when applicable, and a blocker line for needs-attention or run-failed proposals.
3. Use status, type, risk, source, target, unread-only, and search filters; confirm the list updates without a new backend fetch loop.
4. Select rows and run batch approve, reject, read, unread, and defer actions; confirm illegal approve/reject actions are disabled for terminal states.
5. Use `/`, `J`, `K`, `Enter`, `A`, `E`, `R`, `D`, and `Esc` outside text inputs; confirm shortcuts do not fire while focus is inside search/filter inputs.
6. Force loading, empty, no-match, and backend error states; confirm each displays visible text and an actionable retry where applicable.

## Result

- Automated coverage passed for the core filter, keyboard, and illegal batch-action scenarios.
- Manual GUI smoke was not performed because no dev server was started in this validation pass.
