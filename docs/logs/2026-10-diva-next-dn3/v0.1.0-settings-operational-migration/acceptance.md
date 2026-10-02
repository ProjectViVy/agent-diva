# Acceptance steps (owner)

1. `pnpm --dir agent-diva-gui test` → 434/434; `pnpm build` clean.
2. Open the packaged shell → Settings:
   - Providers: select model, enter key → save → reopen → masked key,
     provider shows configured/ready.
   - MCP: add a server → toggle enabled → reopen → persisted.
   - Channels: toggle enabled, edit allow_from → reopen → envelope
     persisted; running state shown via inspect, pending-restart flagged.
   - Cron: create job → toggle → trigger/stop → reopen → persisted.
   - Sandbox: switch preset / domains / approval timeout → reopen → kept.
   - Console: token stats render per period.
3. Confirm no laputa/evolution/persona/memory/notebook/mask/audit nav
   entries remain; pet entry stays (dormant, DN-6).
4. General → danger zone wipe clears local UI prefs only.
