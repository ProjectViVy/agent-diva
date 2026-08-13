# Summary

- Converted `agent-diva-nano` from workspace-only source reuse into a real standalone crate source tree.
- Copied the manager control-plane modules into `agent-diva-nano/src/` so nano no longer depends on `#[path = "../../agent-diva-manager/src/..."]`.
- Added publish-oriented package metadata to `agent-diva-nano`.
- Added `path + version` internal dependency declarations for the nano publish closure crates and the optional CLI nano dependency.
- Preserved the route split:
  - default `full` CLI still uses the original inlined `run_gateway`
  - `nano` remains an explicit independent feature path

# Impact

- `agent-diva-nano` no longer depends on cross-crate `#[path = ...]` source reuse, but at this iteration it still retained runtime-level coupling to `agent-diva-manager`.
- The repository is prepared for topo-ordered crates.io publishing work.
- Default full CLI behavior remains separated from nano.
