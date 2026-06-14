# Summary

- Added a new workspace crate `agent-diva-nano` as the nano runtime/control-plane entry.
- Reused the existing manager implementation source via module path inclusion so the nano crate can compile without physically moving or deleting `agent-diva-manager`.
- Added CLI feature routing:
  - default `full`
  - optional `nano`
- Switched `gateway run` to call the nano runtime entry so both full and nano builds use the same gateway startup path.
- Preserved the existing binary name `agent-diva`, default port `3000`, and current HTTP/SSE behavior.

# Impact

- New crate-level integration point for future nano work.
- No destructive changes to `agent-diva-manager`.
- Full and nano builds can both compile.
