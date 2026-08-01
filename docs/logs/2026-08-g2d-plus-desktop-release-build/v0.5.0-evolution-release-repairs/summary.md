# Evolution release repairs

## Scope

This iteration repairs the desktop Evolution path found during G2D+ acceptance:

- release Tauri builds compile the development-tools calls only in debug builds;
- Recall-feedback replies use the standard `status: ok` envelope;
- persisted AutoDream runs load on every Evolution refresh and timestamps render in the local locale;
- provider reflection accepts both the original complete typed response and a bounded provider-facing response embedded in prose or a JSON fence.

The bounded response contains only candidate intent and input evidence IDs. The manager reconstructs scope and evidence references locally, so a provider cannot set those authority-bound fields.

## Boundaries

No proposal approval, typed Memory write, external provider request, secret exposure, source-profile mutation, deployment, or push was performed by this iteration.
