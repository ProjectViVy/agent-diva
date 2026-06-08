# Summary

## Changed

- Split the Diva pet settings panel into three tabs: 外观, VRM 模型, 动画.
- Added the built-in default appearance preset `default` named 默认角色, bound to `/vrm/models/Alice.vrm`.
- Kept the default appearance out of persisted `vrmAppearances` while always rendering it first in appearance selection.
- Made the default appearance non-editable and non-deletable.
- Moved `.vrm` import and custom model deletion into the VRM 模型 tab.
- Added fallback logic so missing or invalid active appearances resolve to the default Alice preset.
- Updated the desktop pet context menu to use the same default appearance fallback.
- Cleaned visible Chinese copy in the touched settings/menu surfaces.

## Impact

- Existing custom appearances remain stored as user presets.
- Empty appearance configuration now still presents a selectable default role.
- Deleting the active user appearance switches the pet back to Alice.
