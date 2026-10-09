# Engineering acceptance

Review the isolated DIVA branch `feat/issue32-desktop-gates` and the paired
VIVY branch `feat/issue32-remediation`.

- Confirm `VivyCall`, `DesktopDispatch`, and `MediaToken` all call the shared
  native-window capability gate before validation or side effects.
- Confirm the new rejection cases cover no identity, foreign identity, nil,
  unbound, and revoked capability, and that valid calls retain timeout and
  typed upstream errors.
- Confirm the local red/green harness and DIVA GUI client test results in
  `verification.md`.
- Keep canonical Linux host race/native acceptance pending until it runs with
  GTK3 and WebKit2GTK 4.1 development packages. Keep Windows and real voice
  acceptance pending their required environments.

This is an engineering iteration record, not product-owner acceptance. P1
does not mark any native voice, Windows, candidate, or release row as passed.
