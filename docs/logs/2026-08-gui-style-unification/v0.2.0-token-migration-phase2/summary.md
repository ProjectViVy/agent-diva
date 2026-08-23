# GUI Style Unification Phase 2 — Token Migration Summary

**Branch**: `feat/gui-style-phase2` (worktree `agent-diva-style-phase2`)
**Base**: `dev` @ `1471f1e6` (post pet→mate rename)
**Commits**: 8 (token extension + 7 batches + 1 residual fix)
**Files changed**: ~30 component files + styles.css
**Total replacements**: ~490+ color/spacing tokenizations

## Commits

| SHA | Message |
|-----|---------|
| `5bfece14` | feat(gui): extend semantic and identity design tokens |
| `dcd05eb3` | feat(gui): tokenize settings subtree colors |
| `46258090` | refactor(gui): converge conversation sidebar theme overrides into tokens |
| `a7def09b` | feat(gui): tokenize chat planning approval card colors |
| `5d32afbf` | feat(gui): tokenize console views and welcome wizard identity palette |
| `82cefd4f` | feat(gui): tokenize mate overlay glass palette |
| `267e1ec2` | feat(gui): migrate exact-scale font sizes and spacing to tk tokens |
| `cb6c253f` | fix(gui): tokenize residual #dc2626 in stop-btn and update gray-utility bridge comment |

## What was done

### ① Remaining component color tokenization
Migrated ~38 components from hardcoded `#hex/rgba()` to `var(--token, fallback)` with exact-match fallback chains. All semantic colors (danger/success/warning/info/overlay/text/line/panel/accent) now reference design tokens.

### ② Mate overlay glass palette
Converged `rgba(255,255,255,X)` glass-palette values in DesktopMateOverlay/DivaMateView/DivaMateVoicePanel to theme-independent `--mate-*` tokens. Non-exact alpha values preserved for zero visual regression.

### ③ Scoped `.theme-*` override convergence
Deleted 127-line scoped override section in ConversationSidebar (24 overrides across 4 themes). Created 13 `--conv-*` tokens per theme block. Also converged ConfigEditor `.theme-dark .saving-overlay`.

### ④ WelcomeWizard identity palette
25 pink identity literals (`#be185d/#9d174d/#6b2737`) migrated to `--identity-strong/--identity-deep/--identity-ink` tokens. Theme-independent (same value across all 4 themes).

### ⑤ Deep contrast semantic tokens
Added `--danger-strong/--danger-deep/--warning-strong/--success-strong` to all 4 theme blocks. Light themes use original literal values (zero regression), dark themes use same-hue high-contrast values.

### ⑥ tk-* font-size/spacing migration
368 replacements across 15 files: exact-match font-sizes (12/13/14/16/18/20px) → `var(--font-size-*)`, spacing (4/8/12/16/24/32px) → `var(--space-*)`. Non-matching values preserved.

## Token extensions (styles.css)

### Per-theme tokens added
- `--danger-strong`, `--danger-deep`, `--warning-strong`, `--success-strong`
- `--conv-sidebar-bg/--border`, `--conv-search-*`, `--conv-item-hover-bg/--active-bg`, `--conv-icon-*`, `--conv-new-*`, `--conv-menu-*`
- `--saving-overlay-bg` (dark only, others use fallback)

### Theme-independent tokens added (:root)
- `--identity-strong: #be185d`, `--identity-deep: #9d174d`, `--identity-ink: #6b2737`
- `--mate-glass-border/-subtle/-inset/-bg/-bg-hover`, `--mate-text/-muted/-faint`

## Preserved by design
- ThemeSettings.vue preview palette data (26 literals — describes themes, not runtime styles)
- Neutral black shadows, gradient color stops
- Non-matching alpha values and unique color shades
- Gray-utility bridge in styles.css (with updated phase-3 retirement comment)
- `theme-${themeMode}` DOM class on NormalMode/ChatView/AppDialogLayer (still consumed by gray-utility bridge)

## Phase 3 backlog
- Non-matching font-size values (10/11/15/17/21/22/25/28/32/40px)
- Gray-utility bridge retirement (requires template-side `text-gray-*/bg-white` → semantic tokens)
- `theme-${themeMode}` DOM class removal (tied to gray-utility bridge)
- Low-frequency files with ≤6 exact tk-* matches each
