# GMH-21 Acceptance

1. Confirm Core exports stable normalized record and validation contracts.
2. Confirm only applied Laputa or explicitly selected legacy owner data can be
   assigned applied authority.
3. Confirm unsupported Laputa data produces findings and no records.
4. Confirm migration dry-run performs no writes and execute writes only beneath
   the explicit artifact directory.
5. Confirm retries are idempotent, changed inputs conflict, rollback removes
   only owned artifacts, and source files remain byte-for-byte unchanged.
6. Confirm existing provider, Manager, GUI, Markdown, and Laputa JSON behavior
   is unchanged.
