# Summary — Fix collapsed sidebar icon clip by scrollbar

## Problem

Collapsed rail is ~56px with ~8px horizontal padding → ~40px content.
`.sidebar-nav` uses `overflow-y: auto` + thin scrollbar. When the classic scrollbar appears (or steals width while the thumb may be visually quiet/hidden), fixed 40px nav cells overflow and `overflow-x: hidden` clips icons — chat icon looks “cut by a knife”.

## Fix

- Expanded `.sidebar-nav`: `scrollbar-gutter: stable` so thumb show/hide does not reflow icons.
- Collapsed `.sidebar-nav`: hide scrollbar (wheel scroll still works), no gutter, center icon cells.
- Collapsed nav/menu/group cells: `max-width: 100%`, `overflow: visible`, center alignment.
- SVG icons: `overflow: visible` + explicit max size to avoid stroke edge clipping.

## Files

- `agent-diva-gui/src/styles.css`
