# M6 Release

## Release Method

- No separate deployment was performed in this iteration.
- Changes are source-level GUI updates and documentation logs intended to ship with the next normal workspace build/release.

## Rollout Notes

- Build gate: `npm run build` for `agent-diva-gui`.
- Workspace gate: `just fmt-check`, `just check`, and targeted Rust tests listed in `verification.md`.
- Full `just test` currently has an unrelated skills loading failure that should be resolved or accepted before release gating treats it as blocking.

## Compatibility

- Backend `send_message` payload remains `attachments: string[]`.
- Session history only uses attachment metadata and does not store file bytes/base64.
- Provider raw model ID routing is unchanged.
