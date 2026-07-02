# Release

## Method

- No separate deployment was performed in this iteration.
- The change is released through the normal `agent-diva-gui` build and packaging flow.

## Preconditions

- Run the GUI validation command set before packaging:
  - `npm run test -- VrmAppearancePanel.test.ts vrm-animation-scanner.test.ts appearance-config.test.ts pet-config.test.ts DivaVrmAvatar.test.ts DesktopPetOverlay.test.ts DivaPetModelManager.test.ts`
  - `npm run build`

## Rollback

- Revert the feature commit to restore `appearing` and `greeting` to one-shot classification and remove startup motion configuration from appearances.
