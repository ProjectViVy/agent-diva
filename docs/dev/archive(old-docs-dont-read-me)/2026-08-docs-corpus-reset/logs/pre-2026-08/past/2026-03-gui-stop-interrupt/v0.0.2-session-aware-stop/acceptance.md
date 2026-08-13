# Acceptance

## Steps

1. Open GUI and send a long prompt.
2. Click `Stop` and confirm the current generation stops.
3. Click `Clear Chat` (UI clear), then send a new long prompt.
4. Click `Stop` again.

## Expected

- Each stop request affects only the currently active chat identity.
- Stop still does not reset server-side history automatically unless a separate reset flow is used.
