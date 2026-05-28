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
import {
  DEFAULT_APPEARANCE_ID,
  DEFAULT_VRM_APPEARANCE,
  resolveAppearance,
} from '../utils/default-appearance'

export interface AppearanceConfigApi {
  /** All saved appearances (reactive) */
  appearances: Ref<VrmAppearanceConfig[]>
  /** Currently active appearance ID (reactive) */
  activeId: Ref<string>

  /** Create a new appearance and persist */
  createAppearance(appearance: VrmAppearanceConfig): void
  /** Update an existing appearance by ID */
  updateAppearance(id: string, patch: Partial<VrmAppearanceConfig>): void
  /** Delete a user appearance by ID. The built-in default cannot be deleted. */
  deleteAppearance(id: string): void
  /** Switch to a different appearance, returns true if switch happened */
  switchAppearance(id: string): boolean
  /** Get the active appearance config, falling back to the built-in default. */
  getActiveAppearance(): VrmAppearanceConfig
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
    if (id === DEFAULT_APPEARANCE_ID) return
    const list = config.value.vrmAppearances.map((a) =>
      a.id === id ? { ...a, ...patch } : a,
    )
    updateConfig({ vrmAppearances: list })
  }

  function deleteAppearance(id: string): void {
    if (id === DEFAULT_APPEARANCE_ID) return

    const list = config.value.vrmAppearances.filter((a) => a.id !== id)
    const nextActive =
      config.value.activeAppearanceId === id
        ? DEFAULT_APPEARANCE_ID
        : config.value.activeAppearanceId

    updateConfig({ vrmAppearances: list, activeAppearanceId: nextActive })
  }

  function switchAppearance(id: string): boolean {
    if (id === DEFAULT_APPEARANCE_ID) {
      if (config.value.activeAppearanceId === DEFAULT_APPEARANCE_ID) return false
      updateConfig({ activeAppearanceId: DEFAULT_APPEARANCE_ID })
      return true
    }

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

  function getActiveAppearance(): VrmAppearanceConfig {
    return resolveAppearance(config.value.vrmAppearances, config.value.activeAppearanceId)
  }

  function findAppearance(id: string): VrmAppearanceConfig | undefined {
    if (id === DEFAULT_APPEARANCE_ID) return DEFAULT_VRM_APPEARANCE
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
