# GMH-12 Release

No deployment, configuration change, data import, storage rewrite, feature flag,
Manager route, or GUI release is required. The new table is created only when a
caller explicitly constructs `SqliteGovernanceLedger`; no production caller
does so in GMH-12.

Rollback is a normal revert of the focused commit. Existing Plan approvals,
Sandbox approval coordination, and persistent command rules remain authoritative
until a later integration story explicitly migrates them.
