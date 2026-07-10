# Summary — Sidebar Nav Icon Size & Style Polish

## What changed

Polished main sidebar navigation icons for clearer hierarchy and a lighter visual weight.

### Tokens (`styles.css` `:root`)

| Token | Value | Role |
|-------|-------|------|
| `--nav-icon-size` | 18px | Primary nav / group header |
| `--nav-icon-size-sub` | 16px | Sub-items |
| `--nav-icon-size-chevron` | 14px | Expand/collapse chevron |
| `--nav-icon-size-menu` | 18px | Menu toggle |
| `--nav-icon-stroke` | 1.75 | Default stroke |
| `--nav-icon-stroke-active` | 2 | Active stroke |
| `--nav-icon-opacity` | 0.88 | Quiet idle icons |

### Behavior

- Primary icons: 24px → **18px**, thinner stroke
- Sub-items (`.nav-item-sub`): **16px**, muted color, slightly tighter padding
- Chevron: **14px**, lower opacity
- Active: brand color + full opacity + slightly thicker stroke
- Collapsed hit target stays **40×40** with 18px icons (better breathing room)
- Popup menu icons remain **14px**

## Impact

- Files: `agent-diva-gui/src/styles.css`
- No logic / routing / i18n changes
- Themes inherit size tokens from `:root`; color tokens unchanged
