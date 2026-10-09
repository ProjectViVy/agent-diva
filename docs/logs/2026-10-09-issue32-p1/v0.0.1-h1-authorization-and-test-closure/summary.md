# Issue #32 P1.1 authorization and test-closure summary

Date: 2026-10-09. Status: implementation delivered on isolated branches;
canonical native host acceptance remains pending the Linux runner.

The desktop host now shares one native main-window authorization check across
`VivyCall`, `DesktopDispatch`, and `MediaToken`. Missing, foreign, unbound, or
revoked capabilities fail with the closed `-32081` envelope before method
validation, host calls, dispatch handlers, or token return. Authorized RPC
calls retain timeout propagation and upstream error envelopes.

The required host-test path had two earlier blockers. VIVY SDK now maps DIVA's
host-local Laputa aliases to the same locked Laputa snapshot staged for the
consumer module. DIVA test mode now gives SDK pack a minimal embedded
`index.html`, so the frontend is not required to run the headless Go gate.

The focused commits are:

- VIVY `9ba59caa` — map host Laputa replacements to the locked snapshot.
- DIVA `b6ad16e8` — supply the empty embedded page for host tests.
- DIVA `833daa8e` — authorize all privileged desktop bindings.

No public release or upload was performed. P1.2's broader CI/boundary/seal
gates and P1.3's strict candidate/publication gate remain open.
