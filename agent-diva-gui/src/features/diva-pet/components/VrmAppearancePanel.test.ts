import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import VrmAppearancePanel from './VrmAppearancePanel.vue'
import type { VrmAppearanceConfig, VrmModelInfo, VrmMotionInfo } from '../types'

const models: VrmModelInfo[] = [
  { id: 'Alice', name: 'Alice', path: '/vrm/models/Alice.vrm', source: 'builtin' },
  { id: 'custom', name: 'Custom', path: 'vrm/models/custom/custom.vrm', source: 'custom' },
]

const motions: VrmMotionInfo[] = [
  { id: 'idle', name: 'Idle', kind: 'idle', path: '/vrm/animations/idle.vrma' },
]

function mountPanel(overrides: {
  appearances?: VrmAppearanceConfig[]
  activeAppearanceId?: string
} = {}) {
  return mount(VrmAppearancePanel, {
    props: {
      visible: true,
      appearances: overrides.appearances ?? [],
      activeAppearanceId: overrides.activeAppearanceId ?? 'default',
      models,
      motionList: motions,
      inline: true,
    },
  })
}

describe('VrmAppearancePanel', () => {
  it('renders the built-in default appearance when user appearances are empty', () => {
    const wrapper = mountPanel()

    expect(wrapper.text()).toContain('默认角色')
    expect(wrapper.text()).toContain('当前')
    expect(wrapper.text()).not.toContain('暂无外观配置')
  })

  it('does not expose edit or delete actions for the built-in default appearance', () => {
    const wrapper = mountPanel()

    expect(wrapper.find('[aria-label="编辑外观"]').exists()).toBe(false)
    expect(wrapper.find('[aria-label="删除外观"]').exists()).toBe(false)
  })

  it('allows user appearances to be deleted', async () => {
    const wrapper = mountPanel({
      appearances: [
        {
          id: 'custom-appearance',
          name: '自定义角色',
          modelId: 'vrm/models/custom/custom.vrm',
          motionIds: [],
          expressionEnabled: true,
          motionEnabled: true,
        },
      ],
    })

    await wrapper.get('[aria-label="删除外观"]').trigger('click')

    expect(wrapper.emitted('deleteAppearance')?.[0]).toEqual(['custom-appearance'])
  })

  it('falls back to the default selection when activeAppearanceId is missing', () => {
    const wrapper = mountPanel({ activeAppearanceId: 'missing' })

    expect(wrapper.text()).toContain('默认角色')
    expect(wrapper.text()).toContain('当前')
  })
})
