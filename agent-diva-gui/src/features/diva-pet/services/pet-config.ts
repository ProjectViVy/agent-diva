import { ref, watch } from 'vue'
import type { PetConfig } from '../types'
import { DEFAULT_PET_CONFIG, getTtsApiKey } from '../types'
import { loadPetConfigFromCore, savePetConfigToCore } from '../voice/services/voice-api'
import { addVoiceLogEvent } from '../voice/services/voice-log'

const PET_CONFIG_KEY = 'agent-diva-pet-config'
const PET_ASR_DEFAULT_MIGRATION_KEY = 'agent-diva-pet-asr-default-enabled-migrated-v1'
const PET_EXPRESSION_DEFAULT_MIGRATION_KEY = 'agent-diva-pet-expression-default-enabled-migrated-v1'
const DEFAULT_START_MOTION_ID = 'appearing'
let isHydrating = false
let saveRequestId = 0

const isSaving = ref(false)
const lastSaveError = ref<string | null>(null)
const lastSavedAt = ref<number | null>(null)

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
  } else {
    migrated.vrmAppearances = migrated.vrmAppearances.map((appearance) => ({
      ...appearance,
      startMotionId: appearance.startMotionId || DEFAULT_START_MOTION_ID,
    }))
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

  // New GUI logic no longer uses the legacy shared TTS API key.
  migrated.ttsApiKey = null

  return migrated as PetConfig
}

export function mergeCoreConfigWithFrontendConfig(
  coreConfig: Partial<PetConfig>,
  frontendConfig: Partial<PetConfig>,
): PetConfig {
  const migratedCoreConfig = migrateConfig(coreConfig)
  const migratedFrontendConfig = migrateConfig(frontendConfig)

  return migrateConfig({
    ...migratedCoreConfig,
    gaussSceneList: migratedFrontendConfig.gaussSceneList,
    selectedGaussSceneId: migratedFrontendConfig.selectedGaussSceneId,
  })
}

export function applyExpressionDefaultMigration(
  currentConfig: PetConfig,
  getItem: (key: string) => string | null,
  setItem: (key: string, value: string) => void,
): PetConfig {
  if (getItem(PET_EXPRESSION_DEFAULT_MIGRATION_KEY)) {
    return currentConfig
  }

  const migratedConfig = {
    ...currentConfig,
    vrmExpressionEnabled: true,
  }

  setItem(PET_EXPRESSION_DEFAULT_MIGRATION_KEY, '1')
  return migratedConfig
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
    const requestId = ++saveRequestId
    isSaving.value = true
    lastSaveError.value = null
    void savePetConfigToCore(newVal)
      .then(() => {
        if (requestId !== saveRequestId) return
        lastSavedAt.value = Date.now()
      })
      .catch((e: unknown) => {
        console.warn('[pet-config] Failed to save core config:', e)
        if (requestId !== saveRequestId) return
        lastSaveError.value = String(e)
        addVoiceLogEvent({
          level: 'error',
          source: 'settings',
          message: '保存 Diva 语音配置失败',
          detail: { error: String(e) },
        })
      })
      .finally(() => {
        if (requestId === saveRequestId) {
          isSaving.value = false
        }
      })
  }
}, { deep: true })

async function hydrateFromCoreConfig(): Promise<void> {
  try {
    const coreConfig = await loadPetConfigFromCore()
    const shouldEnableAsrByMigration = !localStorage.getItem(PET_ASR_DEFAULT_MIGRATION_KEY)
    isHydrating = true
    config.value = mergeCoreConfigWithFrontendConfig(coreConfig, config.value)
    const expressionMigratedConfig = applyExpressionDefaultMigration(
      config.value,
      (key) => localStorage.getItem(key),
      (key, value) => localStorage.setItem(key, value),
    )
    if (expressionMigratedConfig !== config.value) {
      config.value = expressionMigratedConfig
      localStorage.setItem(PET_CONFIG_KEY, JSON.stringify(expressionMigratedConfig))
      await savePetConfigToCore(expressionMigratedConfig)
    }
    if (shouldEnableAsrByMigration) {
      const migratedConfig = {
        ...config.value,
        asrEnabled: true,
      }
      config.value = migratedConfig
      localStorage.setItem(PET_CONFIG_KEY, JSON.stringify(migratedConfig))
      await savePetConfigToCore(migratedConfig)
      localStorage.setItem(PET_ASR_DEFAULT_MIGRATION_KEY, '1')
      addVoiceLogEvent({
        level: 'info',
        source: 'settings',
        message: '已应用 ASR 默认开启迁移',
      })
    }
    addVoiceLogEvent({
      level: 'info',
      source: 'settings',
      message: '已加载 Diva 语音配置',
      detail: {
        provider: config.value.ttsProvider,
        model: config.value.ttsModel,
        hasApiKey: !!getTtsApiKey(config.value),
      },
    })
  } catch (e) {
    console.warn('[pet-config] Failed to load core config:', e)
    lastSaveError.value = String(e)
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

export function usePetConfigSaveState() {
  return {
    isSaving,
    lastSaveError,
    lastSavedAt,
  }
}

/** Non-reactive read of pet config (for use outside Vue components) */
export function getPetConfigSnapshot(): PetConfig {
  return { ...config.value }
}
