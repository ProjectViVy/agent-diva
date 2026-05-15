/**
 * Multi-appearance configuration manager.
 *
 * Handles CRUD for VrmAppearanceConfig objects and tracks the active
 * appearance.  Works with PetConfig to persist appearance state.
 *
 * Adapted from super-agent-party's VRMConfig name-preset system.
 */

import { computed, type Ref } from 'vue'
import type { PetConfig, VrmAppearanceConfig } from '../types'
import { DEFAULT_PET_CONFIG } from '../types'

export interface AppearanceConfigApi {
  /** All saved appearances (reactive) */
  appearances: Ref<VrmAppearanceConfig[]>
  /** Currently active appearance ID (reactive) */
  activeId: Ref<string>

  /** Create a new appearance and persist */
  createAppearance(appearance: VrmAppearanceConfig): void
  /** Update an existing appearance by ID */
  updateAppearance(id: string, patch: Partial<VrmAppearanceConfig>): void
  /** Delete an appearance by ID (refuses to delete the last one) */
  deleteAppearance(id: string): void
  /** Switch to a different appearance, returns true if switch happened */
  switchAppearance(id: string): boolean
  /** Get the active appearance config (or undefined if none) */
  getActiveAppearance(): VrmAppearanceConfig | undefined
  /** Find appearance by ID */
  findAppearance(id: string): VrmAppearanceConfig | undefined
}

/**
 * Generate a unique appearance ID.
 */
export function generateAppearanceId(): string {
  return `appearance-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`
}

/**
 * Create an empty appearance config with sensible defaults.
 */
export function createEmptyAppearance(name: string, modelId: string): VrmAppearanceConfig {
  return {
    id: generateAppearanceId(),
    name,
    modelId,
    motionIds: [],
    expressionEnabled: true,
    motionEnabled: true,
  }
}

/**
 * Hook-based multi-appearance config manager.
 *
 * @param config  - Reactive PetConfig ref from usePetConfig()
 * @param updateConfig  - PetConfig patcher from usePetConfig()
 */
export function useAppearanceConfig(
  config: Ref<PetConfig>,
  updateConfig: (patch: Partial<PetConfig>) => void,
): AppearanceConfigApi {
  const appearances = computedAppearances(config)
  const activeId = computedActiveId(config)

  function createAppearance(appearance: VrmAppearanceConfig): void {
    const list = [...config.value.vrmAppearances, appearance]
    updateConfig({ vrmAppearances: list })
  }

  function updateAppearance(id: string, patch: Partial<VrmAppearanceConfig>): void {
    const list = config.value.vrmAppearances.map((a) =>
      a.id === id ? { ...a, ...patch } : a,
    )
    updateConfig({ vrmAppearances: list })
  }

  function deleteAppearance(id: string): void {
    const list = config.value.vrmAppearances.filter((a) => a.id !== id)
    if (list.length === 0) {
      console.warn('[appearance-config] Cannot delete the last appearance, resetting to default')
      updateConfig({ vrmAppearances: [], activeAppearanceId: DEFAULT_PET_CONFIG.activeAppearanceId })
      return
    }
    // If we deleted the active appearance, switch to the first remaining one
    const nextActive =
      config.value.activeAppearanceId === id
        ? list[0].id
        : config.value.activeAppearanceId

    updateConfig({ vrmAppearances: list, activeAppearanceId: nextActive })
  }

  function switchAppearance(id: string): boolean {
    const found = config.value.vrmAppearances.find((a) => a.id === id)
    if (!found) {
      console.warn(`[appearance-config] Appearance not found: ${id}`)
      return false
    }
    if (config.value.activeAppearanceId === id) {
      return false // already active
    }
    console.log(`[appearance-config] Switching to appearance: ${found.name}`)
    updateConfig({ activeAppearanceId: id })
    return true
  }

  function getActiveAppearance(): VrmAppearanceConfig | undefined {
    return config.value.vrmAppearances.find(
      (a) => a.id === config.value.activeAppearanceId,
    )
  }

  function findAppearance(id: string): VrmAppearanceConfig | undefined {
    return config.value.vrmAppearances.find((a) => a.id === id)
  }

  return {
    appearances,
    activeId,
    createAppearance,
    updateAppearance,
    deleteAppearance,
    switchAppearance,
    getActiveAppearance,
    findAppearance,
  }
}

// ── Internal computed helpers ──────────────────────────────────

function computedAppearances(config: Ref<PetConfig>) {
  return computed(() => config.value.vrmAppearances)
}

function computedActiveId(config: Ref<PetConfig>) {
  return computed(() => config.value.activeAppearanceId)
}
