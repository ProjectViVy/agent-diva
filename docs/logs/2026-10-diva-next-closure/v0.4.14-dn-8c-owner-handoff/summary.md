# v0.4.14 — DN-8C owner handoff

Prepared the identifiable closure candidate and the owner acceptance
package; the Windows x64 build/run gate is the only remaining engineering
row and it now executes in CI on push.

- `docs/plans/diva-next/closure/owner-acceptance.md`: candidate pins
  (sha256 `d0155e26`, generation `331bb89d`, ABI v1, 20 cognitive
  actions), setup steps incl. `vivy-shared.dll` staging, 26-row scenario
  checklist, OBS09-F1..F5 gap list, bounded diagnostic export procedure.
- `.github/workflows/ci.yml`: new `windows-shell-check` job —
  `cargo check -p vivy-bridge -p diva-speech`, `cargo check
  --workspace`, `cargo test -p vivy-bridge` on windows-latest.
- Candidate version `0.4.13` across tauri.conf / package.json /
  Cargo.toml / Cargo.lock.
- index.md: OBS-09/DN-M-C "Evidence landed", DN-8C "Handoff ready".
