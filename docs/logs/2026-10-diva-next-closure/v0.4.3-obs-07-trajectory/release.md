# Release — OBS-07

Commit: `feat(trajectory): mount authoritative diva activity` on
`feat/dn-closure-wave1`. No push, no PR, no release artifacts.

New/changed:
- `agent-diva-gui/src/state/vivy-trajectory.ts` (new): projection + view.
- `agent-diva-gui/src/state/chat-instance.ts` (new): shared owner instance.
- `agent-diva-gui/src/state/vivy-chat.ts`: owner forwards run events +
  gap marks, fenced/debounced trajectory refetch, session-switch fencing.
- `agent-diva-gui/src/components/console/TrajectoryPanel.vue` (new).
- `agent-diva-gui/src/components/ConsoleView.vue`: Session Activity section.
- `agent-diva-gui/src/api/vivy/{contracts,client,observability}.ts`:
  trajectory + child/list types and calls.
- locales en/zh: `trajectory.*` keys.

Known boundary: bounded run window only; no historical paging. Child refs
are chips, not navigation targets yet.
