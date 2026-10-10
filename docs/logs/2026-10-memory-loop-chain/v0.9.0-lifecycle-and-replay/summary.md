# Public lifecycle and accepted capture replay

VIVY 7a1086a5 adds actual App/public-control correction, tombstone, owned-process restart, and original committed receipt checks. Actual ObserverHost redelivery after three controlled task-owned cursor rewinds preserves accepted source/canonical IDs, body/hash/revision/count; same event with changed content returns event_conflict.

No production change. Native create intentionally mints a fresh record ID; a fresh human create is a new operation and leaves the original tombstone intact. Public API checks are not corrected/deleted facts in later Agent input. S08 recall remains local implementation work; no formal Story status changes.
