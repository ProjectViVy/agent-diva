import { describe, it, expect } from 'vitest'
import { mergeCoreConfigWithFrontendConfig, migrateConfig, migrateLegacyStorageKeys } from './mate-config'

describe('migrateConfig', () => {
  it('旧配置（无新字段）自动填充默认值', () => {
    const oldConfig = { enabled: true, vrmModel: 'test.vrm' }
    const migrated = migrateConfig(oldConfig)

    expect(migrated.desktopMateScale).toBe(1.0)
    expect(migrated.desktopMateAlwaysOnTop).toBe(true)
    expect(migrated.subtitleEnabled).toBe(true)
  })

  it('新字段保留用户设置值', () => {
    const customConfig = {
      desktopMateScale: 1.2,
      desktopMateAlwaysOnTop: false,
      subtitleEnabled: false,
    }
    const migrated = migrateConfig(customConfig)

    expect(migrated.desktopMateScale).toBe(1.2)
    expect(migrated.desktopMateAlwaysOnTop).toBe(false)
    expect(migrated.subtitleEnabled).toBe(false)
  })

  it('desktopMateScale 为 undefined 时回退到默认值', () => {
    const migrated = migrateConfig({ desktopMateScale: undefined as unknown as number })

    expect(migrated.desktopMateScale).toBe(1.0)
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

describe('startup motion migration', () => {
  it('fills appearing when an older appearance has no startMotionId', () => {
    const migrated = migrateConfig({
      vrmAppearances: [
        {
          id: 'legacy',
          name: 'Legacy',
          modelId: 'legacy.vrm',
          motionIds: [],
          expressionEnabled: true,
          motionEnabled: true,
        },
      ],
    })

    expect(migrated.vrmAppearances[0].startMotionId).toBe('appearing')
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

describe('legacy pet storage key migration', () => {
  it('把旧 agent-diva-pet-config 键一次性迁移到 mate 键', () => {
    localStorage.clear()
    localStorage.setItem('agent-diva-pet-config', JSON.stringify({ enabled: false }))
    localStorage.setItem('agent-diva-pet-asr-default-enabled-migrated-v1', '1')

    migrateLegacyStorageKeys()

    expect(localStorage.getItem('agent-diva-mate-config')).toBe(JSON.stringify({ enabled: false }))
    expect(localStorage.getItem('agent-diva-mate-asr-default-enabled-migrated-v1')).toBe('1')
  })

  it('新键已存在时不覆盖用户当前配置', () => {
    localStorage.clear()
    localStorage.setItem('agent-diva-mate-config', JSON.stringify({ enabled: true }))
    localStorage.setItem('agent-diva-pet-config', JSON.stringify({ enabled: false }))

    migrateLegacyStorageKeys()

    expect(JSON.parse(localStorage.getItem('agent-diva-mate-config')!)).toEqual({ enabled: true })
  })
})
