# v0.4.5 acceptance

Pending owner acceptance. Evidence for review:

1. `crates/diva-speech` holds every rule (config CAS, credential slots,
   asset bounds/leases) — `cargo test -p diva-speech` 7/7 green on this
   VM; clippy clean.
2. Secrets: fixed keyring slots `dev.projectivy.diva.speech` /
   `v1.<provider>`; presence-only readback; no plaintext/env/sample
   fallback — `credential_unavailable_no_fallback` proves.
3. Assets: `asset_traversal_rejected` (symlink swap + `..` ids +
   unknown ids), `asset_bounds` (signature/size/count caps, dedup),
   `leased_delete_pending` (pending → completes on lease release).
4. CAS: `config_revision_conflict` (stale base rejected; corrupt file =
   explicit error).
5. Shell: 8 commands registered behind main-window gate; boundary gate
   allowlist updated same commit; no other handler added.

Deferred to DN-8C: full shell compile (`just shell-clippy`), real OS
keyring roundtrip on a host with a store, Windows package smoke.
