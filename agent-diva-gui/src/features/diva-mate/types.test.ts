import { describe, expect, it } from 'vitest'
import { DEFAULT_MATE_CONFIG, type MateConfig } from './types'

describe('DEFAULT_MATE_CONFIG', () => {
  it('should include all existing base fields', () => {
    expect(DEFAULT_MATE_CONFIG.enabled).toBe(true)
    expect(DEFAULT_MATE_CONFIG.vrmModel).toBe('')
    expect(DEFAULT_MATE_CONFIG.ttsEnabled).toBe(false)
    expect(DEFAULT_MATE_CONFIG.asrEnabled).toBe(true)
    expect(DEFAULT_MATE_CONFIG.asrProvider).toBe('web_speech')
    expect(DEFAULT_MATE_CONFIG.asrApiKey).toBeNull()
    expect(DEFAULT_MATE_CONFIG.asrBaseUrl).toBe('')
    expect(DEFAULT_MATE_CONFIG.asrModel).toBeNull()
    expect(DEFAULT_MATE_CONFIG.ttsProvider).toBe('browser')
    expect(DEFAULT_MATE_CONFIG.ttsVoiceId).toBeNull()
    expect(DEFAULT_MATE_CONFIG.ttsSpeed).toBe(1.0)
    expect(DEFAULT_MATE_CONFIG.ttsVolume).toBe(1.0)
  })

  it('should include Phase 1 VRM animation fields with safe defaults', () => {
    expect(DEFAULT_MATE_CONFIG.vrmMotionEnabled).toBe(false)
    expect(DEFAULT_MATE_CONFIG.vrmMotionList).toEqual([])
    expect(DEFAULT_MATE_CONFIG.selectedMotionIds).toEqual([])
  })

  it('should include Phase 1 VRM expression fields with safe defaults', () => {
    expect(DEFAULT_MATE_CONFIG.vrmExpressionEnabled).toBe(false)
    expect(DEFAULT_MATE_CONFIG.vrmExpressionIntensity).toBe(0.85)
  })

  it('should include Phase 1 VRM appearance fields with safe defaults', () => {
    expect(DEFAULT_MATE_CONFIG.vrmAppearances).toEqual([])
    expect(DEFAULT_MATE_CONFIG.activeAppearanceId).toBe('default')
  })

  it('should include Phase 1 VRM interaction fields with safe defaults', () => {
    expect(DEFAULT_MATE_CONFIG.vrmAutoHideEnabled).toBe(false)
    expect(DEFAULT_MATE_CONFIG.vrmLookAtEnabled).toBe(false)
  })

  it('should have all new features disabled by default for backward compatibility', () => {
    // All Phase 1 features default to off, avoiding surprise behavior
    // for users upgrading from older versions.
    const cfg = DEFAULT_MATE_CONFIG
    expect(cfg.vrmMotionEnabled || cfg.vrmExpressionEnabled || cfg.vrmLookAtEnabled).toBe(false)
  })

  it('should be able to merge with a partial old config without throwing', () => {
    // Simulate loading an old config that has none of the new fields
    const oldConfig = {
      enabled: true,
      vrmModel: 'Alice.vrm',
      ttsEnabled: true,
      ttsProvider: 'browser' as const,
    }
    // Spread should work because all new fields have defaults
    const merged: MateConfig = { ...DEFAULT_MATE_CONFIG, ...oldConfig }
    expect(merged.vrmModel).toBe('Alice.vrm')
    expect(merged.vrmMotionEnabled).toBe(false)
    expect(merged.vrmMotionList).toEqual([])
    expect(merged.activeAppearanceId).toBe('default')
  })
})

describe('GaussScene 类型定义', () => {
  it('DEFAULT_MATE_CONFIG.selectedGaussSceneId === "transparent"', () => {
    expect(DEFAULT_MATE_CONFIG.selectedGaussSceneId).toBe('transparent')
  })

  it('DEFAULT_MATE_CONFIG.gaussSceneList.length === 4', () => {
    expect(DEFAULT_MATE_CONFIG.gaussSceneList).toHaveLength(4)
  })

  it('每个 GaussSceneEntry 有 id/name/path/isDefault', () => {
    for (const scene of DEFAULT_MATE_CONFIG.gaussSceneList) {
      expect(scene).toHaveProperty('id')
      expect(scene).toHaveProperty('name')
      expect(scene).toHaveProperty('path')
      expect(scene).toHaveProperty('isDefault')
    }
  })

  it('transparent 的 path 为空字符串', () => {
    const transparent = DEFAULT_MATE_CONFIG.gaussSceneList.find(s => s.id === 'transparent')
    expect(transparent?.path).toBe('')
  })
})
