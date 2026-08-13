# GMH-24C acceptance

Automated acceptance confirms typed provider selection, fail-closed store
opening, governed apply replay safety, rollback removal, payload-free health
diagnostics, GUI removal of the retired settings surface, and the clean-break
scan.

Real-desktop acceptance remains deliberately deferred until the running app is
restarted onto the new binary. Then execute and retain IDs, UI outcome and
Manager/Tauri errors for:

1. approve;
2. reject;
3. edit then approve;
4. duplicate click from two windows;
5. restart after authorization;
6. rollback after apply.

Any duplicate execution, unrecoverable state, stale request acceptance or
silent authority fallback fails acceptance. Evolution remains frozen until all
six scenarios pass.
