# Laputa Persona Lifecycle Workspace

Replaced the minimal persona section editor with a lifecycle-oriented desktop workspace.

- Presents the singleton Laputa persona authority, Frozen Core completion, pending governance, and current-session effectiveness.
- Uses JSON as the section contract and requires an explicit reason before creating a governed proposal.
- Supports proposal approval, approve-and-apply, rejection, deferral, editing, audit inspection, and rollback in the persona surface.
- Tracks the actual Frozen Core revisions captured by each active session and releases that projection on reset or deletion.
- Keeps MEMRULES and WORLD read-only and outside persona authority writes.
- Adds no legacy persona discovery, import, migration, fallback, or compatibility UI.

Affected areas: `agent-diva-laputa`, `agent-diva-agent`, `agent-diva-manager`, and `agent-diva-gui`.
