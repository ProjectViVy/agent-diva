# Acceptance

## User-facing checks

1. Open Diva Pet with an empty VRM selection.
   - Expected: it falls back to `Alice.vrm` and loads a model path under `/vrm/models/`.

2. Save/select a model when config currently contains `Alice`.
   - Expected: the avatar loads normally and does not produce a doubled extension path.

3. Save/select a model when config currently contains `Alice.vrm`.
   - Expected: the avatar still loads normally and the selected model remains highlighted in the manager.

4. Open the desktop pet overlay after selecting a VRM model.
   - Expected: the same normalized model path is used there as in the embedded pet view.

5. Reopen the GUI after the selection is persisted.
   - Expected: the model remains selected and does not enter repeated VRM load retries caused by path mismatch.
