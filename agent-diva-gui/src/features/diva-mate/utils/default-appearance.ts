import type { VrmAppearanceConfig, VrmModelInfo } from '../types'

export const DEFAULT_APPEARANCE_ID = 'default'
export const DEFAULT_VRM_MODEL_PATH = '/vrm/models/Alice.vrm'

export const DEFAULT_VRM_APPEARANCE: VrmAppearanceConfig = {
  id: DEFAULT_APPEARANCE_ID,
  name: '默认角色',
  modelId: DEFAULT_VRM_MODEL_PATH,
  motionIds: [],
  startMotionId: 'appearing',
  expressionEnabled: false,
  motionEnabled: false,
}

export function isDefaultAppearanceId(id: string | null | undefined): boolean {
  return id === DEFAULT_APPEARANCE_ID
}

export function withDefaultAppearance(appearances: VrmAppearanceConfig[] = []): VrmAppearanceConfig[] {
  return [
    DEFAULT_VRM_APPEARANCE,
    ...appearances
      .filter((appearance) => appearance.id !== DEFAULT_APPEARANCE_ID)
      .map(withAppearanceDefaults),
  ]
}

export function hasModel(modelId: string, models: VrmModelInfo[]): boolean {
  return models.some((model) => model.path === modelId || model.id === modelId)
}

export function resolveAppearance(
  appearances: VrmAppearanceConfig[],
  id: string | null | undefined,
  models?: VrmModelInfo[],
): VrmAppearanceConfig {
  if (!id || isDefaultAppearanceId(id)) {
    return DEFAULT_VRM_APPEARANCE
  }

  const found = appearances.find((appearance) => appearance.id === id)
  if (!found) {
    return DEFAULT_VRM_APPEARANCE
  }

  if (models && models.length > 0 && !hasModel(found.modelId, models)) {
    return DEFAULT_VRM_APPEARANCE
  }

  return withAppearanceDefaults(found)
}

export function withAppearanceDefaults(appearance: VrmAppearanceConfig): VrmAppearanceConfig {
  return {
    ...appearance,
    startMotionId: appearance.startMotionId || 'appearing',
  }
}
