# Acceptance

1. Start with a workspace containing `BOOTSTRAP.md` and no soul state.
2. Build the system prompt twice; only the first build may seed Bootstrap state.
3. Confirm a state containing only `bootstrap_seeded_at` does not trigger onboarding again.
4. Confirm a corrupt soul state does not get overwritten or trigger onboarding.
5. During ordinary conversations, the agent must not autonomously read or replay `BOOTSTRAP.md`.

