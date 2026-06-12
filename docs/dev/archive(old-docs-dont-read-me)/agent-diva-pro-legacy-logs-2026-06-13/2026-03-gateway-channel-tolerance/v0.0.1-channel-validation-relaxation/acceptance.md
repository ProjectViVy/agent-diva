# Acceptance

1. Keep a config with `channels.discord.enabled = true` and an empty token.
2. Start the gateway.
3. Confirm the gateway process starts successfully.
4. Confirm status/UI still reports Discord as enabled but not ready.
5. Confirm logs show a channel-level warning rather than `failed to load config`.
6. Add a valid Discord token and restart or reload.
7. Confirm the Discord channel initializes successfully.
