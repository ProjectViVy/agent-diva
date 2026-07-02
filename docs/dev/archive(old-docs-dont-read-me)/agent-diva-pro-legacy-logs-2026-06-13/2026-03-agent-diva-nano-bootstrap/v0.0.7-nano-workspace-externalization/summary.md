# v0.0.7 Nano Workspace Externalization

## Summary

- Removed `agent-diva-nano` from the root workspace member list.
- Removed the local `nano` feature path from `agent-diva-cli`.
- Restored the main CLI local gateway path to `agent-diva-manager` only.
- Left the nano source tree outside the main workspace cargo graph under `external/agent-diva-nano/`, ready for manual relocation.
- Disabled the in-repo nano publish helper path and documented that future nano packaging/publish must happen from the external nano repository.

## Resulting Main-Repo Boundary

- Main repo local install/build target: `agent-diva-cli`
- Main repo local gateway runtime: `agent-diva-manager`
- External nano install target: `agent-diva-nano`
- Temporary directory status: `external/agent-diva-nano/` exists as the staging directory before manual move

## Important Behavioral Change

- Main-repo cargo commands must no longer compile nano locally.
- `agent-diva-cli --no-default-features --features nano` is no longer a supported path.
- Older nano bootstrap logs remain historical records from before this externalization step.
