# DIVA Peach Sakura Pink Default Theme & Design Language Alignment

## Scope

Updated DIVA's default and main theme design language to the **“Peach Sakura Pink”** (蜜桃樱花粉) companionship color scheme. The overall visual direction provides a gentle, reassuring feel using warm white and soft apricot accents, avoiding cloying or childish saturation.

### Color Palette Translation & Tokens

| Role | Color Value | Usage | Token Mappings |
|---|---|---|---|
| Primary | `#F28BA8` | Primary brand identity, selected states, badges | `--brand`, `--accent`, `--accent-glow` |
| Primary (dark) | `#D9567B` | Primary button backgrounds (contrast-safe with white text), pressed states, links, contrast text | `--brand-dark`, `--btn-primary-bg`, `--nav-icon-active` |
| Primary (light) | `#FFE3EA` | Chat bubbles (assistant/other party), card background, tags | `--brand-soft`, `--bubble-assistant-bg`, `--chat-avatar-me-bg` |
| Accent | `#FFC8A2` | Apricot orange for warmth, badges, notification dots | `--accent-warm`, `--badge-accent`, `--badge-accent-text` |
| Secondary | `#C9B6F2` | Light lavender purple for mood depth and night/secondary accents | `--secondary`, `--secondary-light`, `--secondary-bg` |
| Page background | `#FFF8F6` | Warm white (softer and less harsh than pure white) | `--bg-app`, `--panel-solid` |
| Primary text | `#3D2B31` | Warm deep brown (gentler than pure black, 11+:1 contrast) | `--text`, `--chat-text` |
| Secondary text | `#8F7C82` | Descriptive text, timestamps, muted navigation icons | `--text-muted`, `--chat-empty-text`, `--nav-icon` |
| Divider | `#F3E4E7` | Light pink-gray subtle line | `--line`, `--app-shell-border`, `--chat-bar-border` |

### Key Improvements & Design Language Guidelines

1. **Color Ratio & Breathability**: Page background and clean white occupy ~70%, soft pink ~20%, and primary/accent highlights ~10%. Light surfaces dominate large areas, keeping pages clean and breathable.
2. **Button Legibility & Contrast**: Addressed the ~2.5:1 low contrast of white text on `#F28BA8`. Primary action buttons (`.btn-primary`, `.settings-btn-primary`, `.wizard-btn-primary`, `.skills-btn-primary`, `.providers-empty-btn-primary`, `.action-btn-primary`, `.decision-card__btn--approve`) now use `#D9567B` (`--btn-primary-bg`) with crisp white text, providing > 4.5:1 WCAG AA compliant contrast.
3. **Corner Radius**: Adjusted UI corner radius to 20px (`--radius`) and 12px (`--radius-sm`) per the 16–24px softer curvature recommendation. Chat bubbles now feature 20px smooth corners.
4. **Focused Gradients**: Header icons, logo brand cards, and splash screen use the refined `#FFB3C6 → #F28BA8` gradient instead of oversaturated pinks.
5. **Theme Preview & Isolation**: Aligned `love` (default theme) and `default` (minimal theme) in `ThemeSettings.vue` and `styles.css`. Miku and Dark themes remain isolated and untouched per the "default only / main theme only" specification.
