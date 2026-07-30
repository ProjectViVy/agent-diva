# Acceptance

Automated acceptance passes when one bounded evidence event produces exactly one
review proposal, one approved apply creates a Recall-visible typed record, one
terminal feedback event is persisted without raw payload, and rollback removes
the record from subsequent Recall.

Legacy projection schema validation remains covered by existing rejection tests.
Shadow remains fail-closed when no valid typed store exists.

This automated proof intentionally precedes and does not replace the final G2D+
real desktop acceptance.
