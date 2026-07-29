# Persistent validated command rules

Sandbox command approval Phase 3 is complete.

- The Manager and production Exec path share a persistent command-rule store at `~/.agent-diva/execpolicy.toml`.
- Only a conservative backend whitelist can produce a global-rule suggestion. Rules use the complete parsed token sequence, so approval does not broaden to unreviewed parameters.
- `approve_global` persists the rule before resuming the original call. Persistence failure leaves both the pending request and in-memory rules unchanged.
- The Manager, Tauri bridge, approval card, and Sandbox settings page support listing, versioned enable/disable, and deletion.
- Arbitrary GUI rule creation remains unavailable by design.
