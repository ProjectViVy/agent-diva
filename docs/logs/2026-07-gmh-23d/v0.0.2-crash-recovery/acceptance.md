# GMH-23D Crash Recovery Acceptance

Automated acceptance:

- [x] A prepared operation can recover an already committed authority outcome.
- [x] A consumed operation replays the original response.
- [x] Retry does not duplicate changelog or audit records.
- [x] Changed idempotency bindings fail closed.
- [ ] Real desktop approve scenario.
- [ ] Real desktop reject scenario.
- [ ] Real desktop edit-then-approve with stale pre-edit request.
- [ ] Two-window duplicate approve-and-apply.
- [ ] Restart after authorization.
- [ ] Apply and rollback with retained request/proposal/audit/rollback IDs.

Any duplicate execution or unrecoverable state blocks G2D and GMH-24A.
