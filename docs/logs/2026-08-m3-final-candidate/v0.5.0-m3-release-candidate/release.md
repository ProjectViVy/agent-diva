# Release

This is an unpushed local release candidate. No deployment, provider call, real
key read, original-profile use or system security change occurred.

The candidate is built at `target/release/agent-diva-gui.exe`. If the operator's
interactive launch is also rejected with OS error 5, stop acceptance and retain
the evidence; signing/trust or endpoint-policy work requires explicit authority
and is not inferred from this Goal.

Rollback is by reverting the focused M3 commits in reverse order. Governance DB
events are append-only and require no destructive rollback; the new GUI and CLI
surfaces can be reverted without schema migration.
