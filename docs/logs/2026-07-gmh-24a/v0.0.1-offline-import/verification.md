# GMH-24A Verification

- `cargo test -p agent-diva-migration`: dry-run determinism/no-write, apply
  replay, rollback, forbidden Mentle path, and unknown format pass.
- `cargo check -p agent-diva-migration -p agent-diva-laputa -p
  agent-diva-core`: passes.
- Configuration tests cover legacy default, shadow parsing, and unknown-mode
  rejection.
- Final workspace gates are recorded after the GMH-24A focused commit.

Reports and manifests contain identifiers, counts, revisions, digests, paths,
and payload-free integrity data only.
