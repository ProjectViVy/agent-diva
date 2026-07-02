# v0.0.2 Nano Crates.io Prep

## Summary

- Switched `.workspace/agent-diva-nano/Cargo.toml` from `version + path`
  internal dependencies to crates.io-style version dependencies.
- Added local-only `patch.crates-io` overrides so the monorepo can still perform
  development-time checks against the local crates.
- Rewrote `.workspace/agent-diva-nano/README.md` to describe crates.io
  consumption instead of monorepo-only path dependency usage.
- Added crate-level publish-facing README files for:
  - `agent-diva-core`
  - `agent-diva-files`
  - `agent-diva-tooling`
  - `agent-diva-providers`
  - `agent-diva-tools`
  - `agent-diva-agent`
- Added explicit `readme = "README.md"` package metadata to the publish-chain
  crates and `agent-diva-nano`.
- Replaced the garbled `agent-diva-files/README.md` with an ASCII publish-ready
  version.
- Corrected `agent-diva-nano` public documentation entry points to better match
  the external consumption story.

## Impact

- No intended runtime behavior change.
- Primary impact is publish readiness, docs clarity, and manifest preparation
  for the nano crates.io chain.
