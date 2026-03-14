# Acceptance

1. Open `Settings -> Providers` and select any provider with at least one model.
2. Hover a normal model row and confirm a `Test Connection` action appears on the right side.
3. Hover a custom model row and confirm the `Test Connection` button appears immediately to the left of the delete button.
4. Click the test button and confirm the model does not become the active model as a side effect.
5. Confirm the button shows a loading state while the probe is running.
6. Confirm a successful probe shows a short success state with latency.
7. Confirm a failed probe shows a short failure state with the returned error summary.
