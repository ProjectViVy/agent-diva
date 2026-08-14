# I1-S2 Persona Home — Acceptance

## Automated acceptance

- Persona remains uninitialized when all five required files are absent and creates no Markdown seeds on open.
- Initialize writes exactly IDENTITY, RELATIONSHIP, REDLINE, USER preferences, and raw WORLD; DREAM/DARK remain absent.
- Incomplete repair cannot overwrite a valid authority.
- Direct saves use CAS and no-op saves do not create revisions.
- Pending requests become accepted/rejected/stale correctly; WORLD user content and R6 entry rules are enforced.
- Frozen Core freezes six bounded Markdown projections per session and never includes WORLD.
- WORLD is a CORE read tool; Persona read/request/update tools are DEFERRED and absent from subagents.
- Chat is gated on the independent server-side Persona status.
- GUI provides seven-file navigation, CodeMirror source, safe Markdown preview, pending review, history diff, and restore-as-new-revision.

## Manual desktop acceptance still required

1. Start the real Tauri desktop and Manager with a fresh config directory.
2. Complete all five setup fields and verify WORLD is persisted verbatim.
3. Simulate one incomplete file and confirm repair exposes only invalid inputs.
4. Edit each of the seven files, switch source/preview, and verify dirty prompts and CAS conflict recovery.
5. Accept/reject one request and restore one historical revision as a new head.
6. Confirm Chat cannot send before Persona is ready and opens immediately after successful setup.
