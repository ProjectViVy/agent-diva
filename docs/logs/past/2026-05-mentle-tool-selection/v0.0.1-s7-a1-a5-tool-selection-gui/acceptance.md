# Acceptance

## User-facing checks

- [ ] General Settings shows Mentle enable toggle and mode selector.
- [ ] Custom mode exposes a tool checklist (discovered or fallback list).
- [ ] Save persists settings across GUI restart.
- [ ] Disabled Mentle produces no Mentle prompt advertising.
- [ ] Enabled read-only/full/custom modes align prompt exposure with assembled tools.

## Engineering checks

- [ ] Config defaults preserve Sprint 6 RC behavior (Mentle off by default).
- [ ] Unknown tool names in `allowed_tools` do not fail startup.
- [ ] Subagents still do not inherit Mentle tools by default.
- [ ] `with_toolset()` remains registry-driven for prompt activation.
