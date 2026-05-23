import { describe, it, expect } from 'vitest'
import { mergeCoreConfigWithFrontendConfig, migrateConfig } from './pet-config'

describe('migrateConfig', () => {
  it('旧配置（无新字段）自动填充默认值', () => {
    const oldConfig = { enabled: true, vrmModel: 'test.vrm' }
    const migrated = migrateConfig(oldConfig)

    expect(migrated.desktopPetScale).toBe(1.0)
    expect(migrated.desktopPetAlwaysOnTop).toBe(true)
    expect(migrated.subtitleEnabled).toBe(true)
  })

  it('新字段保留用户设置值', () => {
    const customConfig = {
      desktopPetScale: 1.2,
      desktopPetAlwaysOnTop: false,
      subtitleEnabled: false,
    }
    const migrated = migrateConfig(customConfig)

    expect(migrated.desktopPetScale).toBe(1.2)
    expect(migrated.desktopPetAlwaysOnTop).toBe(false)
    expect(migrated.subtitleEnabled).toBe(false)
  })

  it('desktopPetScale 为 undefined 时回退到默认值', () => {
    const migrated = migrateConfig({ desktopPetScale: undefined as unknown as number })

    expect(migrated.desktopPetScale).toBe(1.0)
  })

  it('数组字段（vrmMotionList等）也正确回退', () => {
    const migrated = migrateConfig({})

    expect(migrated.vrmMotionList).toEqual([])
  })

  it('保留已保存的 ASR 开关状态', () => {
    const migrated = migrateConfig({ asrEnabled: true, asrLanguage: 'en-US' })

    expect(migrated.asrEnabled).toBe(true)
    expect(migrated.asrLanguage).toBe('en-US')
  })

  it('补齐新增的云 ASR 配置字段默认值', () => {
    const migrated = migrateConfig({})

    expect(migrated.asrApiKey).toBeNull()
    expect(migrated.asrBaseUrl).toBe('')
    expect(migrated.asrModel).toBeNull()
  })

  it('保留已保存的云 ASR 配置', () => {
    const migrated = migrateConfig({
      asrProvider: 'siliconflow',
      asrApiKey: 'test-key',
      asrBaseUrl: 'https://api.siliconflow.cn/v1',
      asrModel: 'FunAudioLLM/SenseVoiceSmall',
    })

    expect(migrated.asrProvider).toBe('siliconflow')
    expect(migrated.asrApiKey).toBe('test-key')
    expect(migrated.asrBaseUrl).toBe('https://api.siliconflow.cn/v1')
    expect(migrated.asrModel).toBe('FunAudioLLM/SenseVoiceSmall')
  })

  it('旧配置缺少 ttsVoiceId 时自动补默认值', () => {
    const migrated = migrateConfig({ ttsProvider: 'minimax' as const })

    expect(migrated.ttsVoiceId).toBeNull()
  })

  it('保留已保存的 ttsVoiceId', () => {
    const migrated = migrateConfig({
      ttsProvider: 'minimax' as const,
      ttsVoiceId: 'male-qn-qingse',
    })

    expect(migrated.ttsVoiceId).toBe('male-qn-qingse')
  })
})

describe('场景配置持久化', () => {
  it('旧配置(无场景字段) → 自动合并默认值', () => {
    const oldConfig = { enabled: true, vrmModel: 'test.vrm' }
    const migrated = migrateConfig(oldConfig)

    expect(migrated.selectedGaussSceneId).toBe('transparent')
    expect(migrated.gaussSceneList).toHaveLength(4)
  })

  it('场景ID不在列表中 → 回退 transparent', () => {
    const config = { selectedGaussSceneId: 'invalid-scene' as any }
    const migrated = migrateConfig(config)

    expect(migrated.selectedGaussSceneId).toBe('transparent')
  })

  it('选择 home → 读取 home', () => {
    const config = { selectedGaussSceneId: 'home' as any }
    const migrated = migrateConfig(config)

    expect(migrated.selectedGaussSceneId).toBe('home')
  })

  it('从核心配置 hydrate 时保留前端专属场景选择', () => {
    const merged = mergeCoreConfigWithFrontendConfig(
      { enabled: false, selectedGaussSceneId: 'transparent' as any },
      { selectedGaussSceneId: 'sea' as any },
    )

    expect(merged.enabled).toBe(false)
    expect(merged.selectedGaussSceneId).toBe('sea')
  })
})
