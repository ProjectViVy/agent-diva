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
  { id: 'appearing', name: 'Appearing', kind: 'startup', path: '/vrm/animations/appearing.vrma' },
  { id: 'greeting', name: 'Greeting', kind: 'startup', path: '/vrm/animations/greeting.vrma' },
  { id: 'liked', name: 'Liked', kind: 'oneshot', path: '/vrm/animations/liked.vrma' },
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
  it('shows startup motions separately and saves the selected startMotionId', async () => {
    const wrapper = mountPanel()

    await wrapper.findAll('button')[0].trigger('click')

    expect(wrapper.text()).toContain('开始动作')
    expect(wrapper.text()).toContain('Appearing')
    expect(wrapper.text()).toContain('Greeting')
    expect(wrapper.text()).not.toContain('Liked')

    const greetingButton = wrapper.findAll('button').find((button) => button.text() === 'Greeting')
    expect(greetingButton).toBeTruthy()
    await greetingButton!.trigger('click')

    const saveButton = wrapper.findAll('button').find((button) => button.classes().includes('bg-pink-500'))
    expect(saveButton).toBeTruthy()
    await saveButton!.trigger('click')

    const event = wrapper.emitted('createAppearance')?.[0]?.[0] as VrmAppearanceConfig
    expect(event.startMotionId).toBe('greeting')
  })
})
