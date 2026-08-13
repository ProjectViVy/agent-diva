# Acceptance

1. Construct an agent with an active Mentle runtime and `memtle_status`.
2. Re-register default tools and rebuild the per-turn tool registry.
3. Confirm Mentle remains active, `memtle_status` remains registered, and the provider-backed `Memory Startup Status` remains in the system prompt.
4. Construct an agent with Mentle inactive and repeat both rebuild paths.
5. Confirm no Mentle tool or Mentle routing text is exposed.
6. Confirm neither path reintroduces the retired `L2 Palace Memory` prompt contract.
