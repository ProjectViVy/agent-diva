import { ref, watch } from 'vue'
import type { PetConfig } from '../types'
import { DEFAULT_PET_CONFIG } from '../types'
import { loadPetConfigFromCore, savePetConfigToCore } from '../voice/services/voice-api'
import { addVoiceLogEvent } from '../voice/services/voice-log'

const PET_CONFIG_KEY = 'agent-diva-pet-config'
let isHydrating = false

/**
 * Auto-migrate config from older format to current schema.
 * Fills in missing fields from DEFAULT_PET_CONFIG without overwriting user values.
 */
export function migrateConfig(config: Partial<PetConfig>): PetConfig {
  // Strip undefined values so they don't override defaults via spread
  const cleanConfig = Object.fromEntries(
    Object.entries(config).filter(([_, v]) => v !== undefined),
  ) as Partial<PetConfig>
  const migrated = { ...DEFAULT_PET_CONFIG, ...cleanConfig }

  // v2.x → v3.x: ensure VRM animation fields exist
  if (!Array.isArray(migrated.vrmMotionList)) {
    migrated.vrmMotionList = DEFAULT_PET_CONFIG.vrmMotionList
  }
  if (!Array.isArray(migrated.selectedMotionIds)) {
    migrated.selectedMotionIds = DEFAULT_PET_CONFIG.selectedMotionIds
  }

  // v2.x → v3.x: ensure VRM appearance fields exist
  if (!Array.isArray(migrated.vrmAppearances)) {
    migrated.vrmAppearances = DEFAULT_PET_CONFIG.vrmAppearances
  }

  // v3.x → v4.x: ensure 3D Gauss scene fields exist
  if (!Array.isArray(migrated.gaussSceneList) || migrated.gaussSceneList.length === 0) {
    migrated.gaussSceneList = DEFAULT_PET_CONFIG.gaussSceneList
  }
  if (!migrated.selectedGaussSceneId) {
    migrated.selectedGaussSceneId = DEFAULT_PET_CONFIG.selectedGaussSceneId
  } else {
    // Validate that the selected ID exists in the list
    const validIds = migrated.gaussSceneList.map(s => s.id)
    if (!validIds.includes(migrated.selectedGaussSceneId)) {
      migrated.selectedGaussSceneId = DEFAULT_PET_CONFIG.selectedGaussSceneId
    }
  }

  return migrated as PetConfig
}

function loadConfig(): PetConfig {
  try {
    const raw = localStorage.getItem(PET_CONFIG_KEY)
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<PetConfig>
      return migrateConfig(parsed)
    }
  } catch (e) {
    console.warn('[pet-config] Failed to load config:', e)
  }
  return { ...DEFAULT_PET_CONFIG }
}

const config = ref<PetConfig>(loadConfig())

void hydrateFromCoreConfig()

watch(config, (newVal) => {
  try {
    localStorage.setItem(PET_CONFIG_KEY, JSON.stringify(newVal))
  } catch (e) {
    console.warn('[pet-config] Failed to save config:', e)
  }

  if (!isHydrating) {
    void savePetConfigToCore(newVal).catch((e: unknown) => {
      console.warn('[pet-config] Failed to save core config:', e)
      addVoiceLogEvent({
        level: 'error',
        source: 'settings',
        message: '保存 Diva 语音配置失败',
        detail: { error: String(e) },
      })
    })
  }
}, { deep: true })

async function hydrateFromCoreConfig(): Promise<void> {
  try {
    const coreConfig = await loadPetConfigFromCore()
    isHydrating = true
    config.value = migrateConfig(coreConfig)
    addVoiceLogEvent({
      level: 'info',
      source: 'settings',
      message: '已加载 Diva 语音配置',
      detail: {
        provider: config.value.ttsProvider,
        model: config.value.ttsModel,
        hasApiKey: !!config.value.ttsApiKey,
      },
    })
  } catch (e) {
    console.warn('[pet-config] Failed to load core config:', e)
    addVoiceLogEvent({
      level: 'warn',
      source: 'settings',
      message: '读取核心配置失败，已使用本地缓存',
      detail: { error: String(e) },
    })
  } finally {
    isHydrating = false
  }
}

/** Reactive pet configuration, backed by localStorage */
export function usePetConfig() {
  function setEnabled(value: boolean) {
    config.value = { ...config.value, enabled: value }
  }

  function updateConfig(patch: Partial<PetConfig>) {
    config.value = { ...config.value, ...patch }
  }

  return { config, setEnabled, updateConfig }
}

/** Non-reactive read of pet config (for use outside Vue components) */
export function getPetConfigSnapshot(): PetConfig {
  return { ...config.value }
}
