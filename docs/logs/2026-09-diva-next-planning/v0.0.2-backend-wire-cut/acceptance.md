# Acceptance — backend wire-cut (DN-W)

1. The branch contains no Rust workspace, no `src-tauri`, and no Cargo/Tauri build path.
2. `pnpm --dir agent-diva-gui test` and `pnpm --dir agent-diva-gui build` pass.
3. The Vue frontend is preserved intact; dead `invoke`/`listen` seams are enumerated for per-domain removal in DN-1..DN-6.
4. The planning package reflects the reordered DAG (DN-W first, DN-8 = final acceptance gate).
