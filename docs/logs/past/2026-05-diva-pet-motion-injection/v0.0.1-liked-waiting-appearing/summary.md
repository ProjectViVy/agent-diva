# Iteration Summary

## Changes

- Registered three new VRMA motions in the Diva Pet runtime catalog:
  - `waiting` as an idle motion.
  - `appearing` as a one-shot motion.
  - `liked` as a one-shot motion.
- Registered the same motion IDs in the GUI VRMA animation scanner so they appear in the animation management list.
- Copied the three VRMA assets into the standalone `avatar-runtime-vrm` public animation directory.
- Updated catalog sync tests to cover the new motion IDs and expected kinds.

## Impact

- Diva Pet can discover and play the new animations through the existing motion management flow.
- Runtime and GUI scanner catalogs remain aligned.
