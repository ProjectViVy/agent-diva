# Release — v0.4.11 DN-P-C

## Commits

- agent-vivy `2f38c2e5` — chore(sdk): repin module source digests after
  dependency tidy drift
- agent-vivy `db7f0c55` — chore(go.mod): require workspace modules imported
  by generated assemblies
- agent-diva `<this commit>` — chore(packaging): stage the DN-P-C closure
  candidate (generation 331bb89d) + governance defaults + inventory refresh

## Artifact channel

The sealed `vivy-shared.so` (sha256 `d0155e26…`, 107,489,640 B) is staged
under `agent-diva-gui/src-tauri/vivy-runtime/` and remains gitignored per
the existing binary policy. Committed provenance: `generation.json`
(generationId `331bb89d…`, recipeDigest `583d2b5a…`), `vivy-shared.h`,
`vivy_abi.h`.

No push, no PR, no release tag — candidate stage only.
