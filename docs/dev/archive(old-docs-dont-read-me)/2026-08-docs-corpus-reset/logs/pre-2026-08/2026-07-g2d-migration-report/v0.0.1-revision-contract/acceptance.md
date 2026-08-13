# Acceptance

For an empty store importing two deterministic records, dry-run and apply must
report expected revision `0`, resulting revision `2`, and integrity revision
`2`. Replay must return the same values without new writes. Rollback must
restore revision `0`; a subsequent reapply must return to revision `2`.
