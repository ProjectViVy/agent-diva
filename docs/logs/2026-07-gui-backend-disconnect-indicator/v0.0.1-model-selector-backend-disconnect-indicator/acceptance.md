# Acceptance

1. Start the GUI and open the normal chat view.
2. With backend healthy, confirm the top-right model selector has no warning icon.
3. Stop or disconnect the backend so health checks enter the error state.
4. Confirm a warning icon appears beside the model selector and its tooltip states that the backend is not connected.
5. Open the model dropdown and confirm the selector still works while the warning icon remains visible.
