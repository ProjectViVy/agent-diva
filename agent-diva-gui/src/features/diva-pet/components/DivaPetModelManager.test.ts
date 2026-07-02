import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import type { PetConfig } from '../types'
import { DEFAULT_PET_CONFIG } from '../types'

const { mockConfig, mockUpdateConfig } = vi.hoisted(() => {
  const mockConfig = { __v_isRef: true, value: null as unknown as PetConfig }
  const mockUpdateConfig = vi.fn((patch: Partial<PetConfig>) => {
    mockConfig.value = { ...mockConfig.value, ...patch }
  })
  return { mockConfig, mockUpdateConfig }
})

vi.mock('../services/pet-config', () => ({
  usePetConfig: () => ({
    config: mockConfig,
    updateConfig: mockUpdateConfig,
  }),
}))

vi.mock('../utils/vrm-animation-scanner', () => ({
  scanVRMAnimations: vi.fn(() => Promise.resolve([])),
  buildKnownMotionInfo: vi.fn(() => []),
}))

vi.mock('lucide-vue-next', () => ({
  AlertCircle: { template: '<span />' },
  Check: { template: '<span />' },
  Circle: { template: '<span />' },
  FolderOpen: { template: '<span />' },
  Loader2: { template: '<span />' },
  PackageOpen: { template: '<span />' },
  Pencil: { template: '<span />' },
  Plus: { template: '<span />' },
  Settings: { template: '<span />' },
  Trash2: { template: '<span />' },
  Upload: { template: '<span />' },
  X: { template: '<span />' },
}))

describe('DivaPetModelManager', () => {
  beforeEach(() => {
    mockConfig.value = {
      ...DEFAULT_PET_CONFIG,
      vrmAppearances: [],
      activeAppearanceId: 'default',
      vrmModel: '',
    }
    mockUpdateConfig.mockClear()
  })

  it('uses three tabs and keeps VRM import on the VRM model tab only', async () => {
    const { default: DivaPetModelManager } = await import('./DivaPetModelManager.vue')
    const wrapper = mount(DivaPetModelManager, {
      props: { visible: true },
      global: {
        stubs: { Teleport: true, Transition: false },
      },
    })

    await nextTick()
    await nextTick()

    expect(wrapper.text()).toContain('外观')
    expect(wrapper.text()).toContain('VRM 模型')
    expect(wrapper.text()).toContain('动画')
    expect(wrapper.text()).toContain('默认角色')
    expect(wrapper.text()).not.toContain('导入 .vrm 模型')

    await wrapper.findAll('button').find((button) => button.text() === 'VRM 模型')!.trigger('click')
    await nextTick()

    expect(wrapper.text()).toContain('导入 .vrm 模型')
  })

  it('falls back to Alice when the active appearance is missing', async () => {
    mockConfig.value.activeAppearanceId = 'missing'
    const { default: DivaPetModelManager } = await import('./DivaPetModelManager.vue')
    mount(DivaPetModelManager, {
      props: { visible: true },
      global: {
        stubs: { Teleport: true, Transition: false },
      },
    })

    await nextTick()
    await nextTick()

    expect(mockConfig.value.activeAppearanceId).toBe('default')
    expect(mockConfig.value.vrmModel).toBe('/vrm/models/Alice.vrm')
  })
})
