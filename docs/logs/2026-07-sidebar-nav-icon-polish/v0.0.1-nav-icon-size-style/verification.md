# Verification

## Automated

- GUI change is CSS-only; workspace `just check` / full Rust suite not required for this slice.
- No new unit tests (presentation tokens only).

## Manual / smoke (GUI)

Suggested checks after `pnpm dev` / app start in `agent-diva-gui`:

1. Expanded sidebar: primary icons ~18px, sub-items ~16px, labels align without clipping
2. Collapsed sidebar: icons centered in 40×40 cells, no overflow
3. Hover / active: color and stroke feedback; sub-items stay quieter when idle
4. Group chevrons smaller than group icons
5. Theme switch love / dark / default / miku: icon colors still follow `--nav-icon*`
6. Collapsed group popup: 14px icons unchanged

## Result

- Implementation complete in CSS; visual smoke deferred to local GUI run when convenient.
