import { ref, watch } from 'vue'
import type { MateConfig } from '../types'
import { DEFAULT_MATE_CONFIG, getTtsApiKey } from '../types'
import { loadMateConfigFromCore, saveMateConfigToCore } from '../voice/services/voice-api'
import { addVoiceLogEvent } from '../voice/services/voice-log'

const MATE_CONFIG_KEY = 'agent-diva-mate-config'
const MATE_ASR_DEFAULT_MIGRATION_KEY = 'agent-diva-mate-asr-default-enabled-migrated-v1'
const MATE_EXPRESSION_DEFAULT_MIGRATION_KEY = 'agent-diva-mate-expression-default-enabled-migrated-v1'
const LEGACY_PET_CONFIG_KEY = 'agent-diva-pet-config'
const LEGACY_PET_ASR_DEFAULT_MIGRATION_KEY = 'agent-diva-pet-asr-default-enabled-migrated-v1'
const LEGACY_PET_EXPRESSION_DEFAULT_MIGRATION_KEY = 'agent-diva-pet-expression-default-enabled-migrated-v1'
const DEFAULT_START_MOTION_ID = 'appearing'
let isHydrating = false
let saveRequestId = 0

const isSaving = ref(false)
const lastSaveError = ref<string | null>(null)
const lastSavedAt = ref<number | null>(null)

/**
 * Auto-migrate config from older format to current schema.
 * Fills in missing fields from DEFAULT_MATE_CONFIG without overwriting user values.
 */
export function migrateConfig(config: Partial<MateConfig>): MateConfig {
  // Strip undefined values so they don't override defaults via spread
  const cleanConfig = Object.fromEntries(
    Object.entries(config).filter(([_, v]) => v !== undefined),
  ) as Partial<MateConfig>
  const migrated = { ...DEFAULT_MATE_CONFIG, ...cleanConfig }

  // v2.x → v3.x: ensure VRM animation fields exist
  if (!Array.isArray(migrated.vrmMotionList)) {
    migrated.vrmMotionList = DEFAULT_MATE_CONFIG.vrmMotionList
  }
  if (!Array.isArray(migrated.selectedMotionIds)) {
    migrated.selectedMotionIds = DEFAULT_MATE_CONFIG.selectedMotionIds
  }

  // v2.x → v3.x: ensure VRM appearance fields exist
  if (!Array.isArray(migrated.vrmAppearances)) {
    migrated.vrmAppearances = DEFAULT_MATE_CONFIG.vrmAppearances
  } else {
    migrated.vrmAppearances = migrated.vrmAppearances.map((appearance) => ({
      ...appearance,
      startMotionId: appearance.startMotionId || DEFAULT_START_MOTION_ID,
    }))
  }

  // v3.x → v4.x: ensure 3D Gauss scene fields exist
  if (!Array.isArray(migrated.gaussSceneList) || migrated.gaussSceneList.length === 0) {
    migrated.gaussSceneList = DEFAULT_MATE_CONFIG.gaussSceneList
  }
  if (!migrated.selectedGaussSceneId) {
    migrated.selectedGaussSceneId = DEFAULT_MATE_CONFIG.selectedGaussSceneId
  } else {
    // Validate that the selected ID exists in the list
    const validIds = migrated.gaussSceneList.map(s => s.id)
    if (!validIds.includes(migrated.selectedGaussSceneId)) {
      migrated.selectedGaussSceneId = DEFAULT_MATE_CONFIG.selectedGaussSceneId
    }
  }

  // New GUI logic no longer uses the legacy shared TTS API key.
  migrated.ttsApiKey = null

  return migrated as MateConfig
}

export function mergeCoreConfigWithFrontendConfig(
  coreConfig: Partial<MateConfig>,
  frontendConfig: Partial<MateConfig>,
): MateConfig {
  const migratedCoreConfig = migrateConfig(coreConfig)
  const migratedFrontendConfig = migrateConfig(frontendConfig)

  return migrateConfig({
    ...migratedCoreConfig,
    gaussSceneList: migratedFrontendConfig.gaussSceneList,
    selectedGaussSceneId: migratedFrontendConfig.selectedGaussSceneId,
  })
}

export function applyExpressionDefaultMigration(
  currentConfig: MateConfig,
  getItem: (key: string) => string | null,
  setItem: (key: string, value: string) => void,
): MateConfig {
  if (getItem(MATE_EXPRESSION_DEFAULT_MIGRATION_KEY)) {
    return currentConfig
  }

  const migratedConfig = {
    ...currentConfig,
    vrmExpressionEnabled: true,
  }

  setItem(MATE_EXPRESSION_DEFAULT_MIGRATION_KEY, '1')
  return migratedConfig
}

/**
 * One-time migration from the legacy `pet`-era localStorage keys to the
 * renamed `mate` keys. Copies values only when the new key is absent so
 * existing users keep their configuration after the rename.
 */
export function migrateLegacyStorageKeys(): void {
  try {
    if (!localStorage.getItem(MATE_CONFIG_KEY)) {
      const legacy = localStorage.getItem(LEGACY_PET_CONFIG_KEY)
      if (legacy) {
        localStorage.setItem(MATE_CONFIG_KEY, legacy)
      }
    }
    const flagPairs: Array<[string, string]> = [
      [MATE_ASR_DEFAULT_MIGRATION_KEY, LEGACY_PET_ASR_DEFAULT_MIGRATION_KEY],
      [MATE_EXPRESSION_DEFAULT_MIGRATION_KEY, LEGACY_PET_EXPRESSION_DEFAULT_MIGRATION_KEY],
    ]
    for (const [nextKey, legacyKey] of flagPairs) {
      if (!localStorage.getItem(nextKey)) {
        const legacyFlag = localStorage.getItem(legacyKey)
        if (legacyFlag) {
          localStorage.setItem(nextKey, legacyFlag)
        }
      }
    }
  } catch (e) {
    console.warn('[mate-config] Failed to migrate legacy storage keys:', e)
  }
}

migrateLegacyStorageKeys()

function loadConfig(): MateConfig {
  try {
    const raw = localStorage.getItem(MATE_CONFIG_KEY)
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<MateConfig>
      return migrateConfig(parsed)
    }
  } catch (e) {
    console.warn('[mate-config] Failed to load config:', e)
  }
  return { ...DEFAULT_MATE_CONFIG }
}

const config = ref<MateConfig>(loadConfig())

void hydrateFromCoreConfig()

watch(config, (newVal) => {
  try {
    localStorage.setItem(MATE_CONFIG_KEY, JSON.stringify(newVal))
  } catch (e) {
    console.warn('[mate-config] Failed to save config:', e)
  }

  if (!isHydrating) {
    const requestId = ++saveRequestId
    isSaving.value = true
    lastSaveError.value = null
    void saveMateConfigToCore(newVal)
      .then(() => {
        if (requestId !== saveRequestId) return
        lastSavedAt.value = Date.now()
      })
      .catch((e: unknown) => {
        console.warn('[mate-config] Failed to save core config:', e)
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
    const coreConfig = await loadMateConfigFromCore()
    const shouldEnableAsrByMigration = !localStorage.getItem(MATE_ASR_DEFAULT_MIGRATION_KEY)
    isHydrating = true
    config.value = mergeCoreConfigWithFrontendConfig(coreConfig, config.value)
    const expressionMigratedConfig = applyExpressionDefaultMigration(
      config.value,
      (key) => localStorage.getItem(key),
      (key, value) => localStorage.setItem(key, value),
    )
    if (expressionMigratedConfig !== config.value) {
      config.value = expressionMigratedConfig
      localStorage.setItem(MATE_CONFIG_KEY, JSON.stringify(expressionMigratedConfig))
      await saveMateConfigToCore(expressionMigratedConfig)
    }
    if (shouldEnableAsrByMigration) {
      const migratedConfig = {
        ...config.value,
        asrEnabled: true,
      }
      config.value = migratedConfig
      localStorage.setItem(MATE_CONFIG_KEY, JSON.stringify(migratedConfig))
      await saveMateConfigToCore(migratedConfig)
      localStorage.setItem(MATE_ASR_DEFAULT_MIGRATION_KEY, '1')
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
    console.warn('[mate-config] Failed to load core config:', e)
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

/** Reactive mate configuration, backed by localStorage */
export function useMateConfig() {
  function setEnabled(value: boolean) {
    config.value = { ...config.value, enabled: value }
  }

  function updateConfig(patch: Partial<MateConfig>) {
    config.value = { ...config.value, ...patch }
  }

  return { config, setEnabled, updateConfig }
}

export function useMateConfigSaveState() {
  return {
    isSaving,
    lastSaveError,
    lastSavedAt,
  }
}

/** Non-reactive read of mate config (for use outside Vue components) */
export function getMateConfigSnapshot(): MateConfig {
  return { ...config.value }
}
