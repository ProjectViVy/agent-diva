# Release Notes — Phase 2 Token Migration

## Summary

Completed the second phase of GUI style unification: migrated ~490+ hardcoded color and spacing literals across ~30 component files to CSS design tokens.

## Key outcomes

1. **All scoped `.theme-*` overrides retired** from ConversationSidebar (127 lines removed)
2. **13 new per-theme `--conv-*` tokens** for conversation sidebar theming
3. **4 new deep-contrast tokens** (`--danger-strong/--danger-deep/--warning-strong/--success-strong`)
4. **3 identity palette tokens** (`--identity-strong/--identity-deep/--identity-ink`)
5. **8 mate glass tokens** (`--mate-glass-*/--mate-text-*`)
6. **1 saving overlay token** (`--saving-overlay-bg`, dark theme only)
7. **368 font-size/spacing values** migrated to `var(--font-size-*)` / `var(--space-*)`
8. **Gray-utility bridge retained** with phase-3 retirement conditions documented

## Build artifacts

- CSS bundle: ~187KB (net reduction from override removal)
- No JS bundle impact (CSS-only changes)
- All 4 themes visually identical to pre-migration state
