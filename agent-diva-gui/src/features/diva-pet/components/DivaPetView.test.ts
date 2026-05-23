import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick, ref } from 'vue'
import type { PetConfig, PetMessage } from '../types'
import { DEFAULT_PET_CONFIG } from '../types'

const { mockVoiceSetEnabled, mockVoiceState, mockSpeakText, mockGetDesktopPetEmotionSignal } = vi.hoisted(() => {
  const mockVoiceState = {
    isEnabled: { value: false },
    isListening: { value: false },
    isProcessing: { value: false },
    error: { value: null as string | null },
  }
  return {
    mockVoiceSetEnabled: vi.fn((enabled: boolean) => {
      mockVoiceState.isEnabled.value = enabled
      return Promise.resolve(true)
    }),
    mockVoiceState,
    mockSpeakText: vi.fn(),
    mockGetDesktopPetEmotionSignal: vi.fn<() => { signature: string; mood: string } | null>(() => null),
  }
})

const mockConfig = ref<PetConfig>(makeMockPetConfig())

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(() => Promise.resolve(undefined)),
}))

vi.mock('./EmbeddedPetFrame.vue', () => ({
  default: {
    name: 'EmbeddedPetFrame',
    template: '<div class="embedded-pet-frame-stub" />',
    props: [
      'modelPath',
      'mood',
      'isSpeaking',
      'active',
      'lipSyncEnabled',
      'backgroundScene',
      'backgroundSceneUrl',
      'transparentBackground',
    ],
  },
}))

vi.mock('../voice/components/DivaPetVoicePanel.vue', () => ({
  default: {
    name: 'DivaPetVoicePanel',
    template: `
      <div class="voice-panel-stub">
        <button
          class="voice-panel-ptt-button"
          :disabled="isPushToTalkDisabled"
          @pointerdown="$emit('startVoiceHold', $event)"
          @pointerup="$emit('stopVoiceHold', $event)"
        >按住说话</button>
      </div>
    `,
    props: [
      'isSpeaking',
      'isVoiceSupported',
      'isVoiceEnabled',
      'isListening',
      'isProcessing',
      'voiceError',
      'ttsEnabled',
      'isPushToTalkDisabled',
    ],
    emits: ['toggleVoice', 'update:ttsEnabled', 'testSpeak', 'stopSpeaking', 'startVoiceHold', 'stopVoiceHold'],
  },
}))

vi.mock('./DivaPetModelManager.vue', () => ({
  default: {
    name: 'DivaPetModelManager',
    template: '<div v-if="visible" class="model-manager-stub" />',
    props: ['visible'],
    emits: ['close', 'modelChanged'],
  },
}))

vi.mock('../services/pet-config', () => ({
  usePetConfig: () => ({
    config: mockConfig,
    setEnabled: vi.fn(),
    updateConfig: vi.fn((patch: Partial<PetConfig>) => {
      mockConfig.value = { ...mockConfig.value, ...patch }
    }),
  }),
}))

vi.mock('lucide-vue-next', () => ({
  Menu: { name: 'Menu', template: '<span class="menu-icon" />' },
  Settings: { name: 'Settings', template: '<span class="settings-icon" />' },
  Send: { name: 'Send', template: '<span class="send-icon" />' },
  Loader2: { name: 'Loader2', template: '<span class="loader-icon" />' },
  Monitor: { name: 'Monitor', template: '<span class="monitor-icon" />' },
  Image: { name: 'Image', template: '<span class="image-icon" />' },
  Mic: { name: 'Mic', template: '<span class="mic-icon" />' },
  Plus: { name: 'Plus', template: '<span class="plus-icon" />' },
}))

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

vi.mock('../../../utils/desktop-pet-emotion', () => ({
  getDesktopPetEmotionSignal: mockGetDesktopPetEmotionSignal,
}))

vi.mock('../utils/vrm-model', () => ({
  resolveVrmModelPath: vi.fn(() => '/mock/path/model.vrm'),
}))

vi.mock('../utils/gauss-scene', () => ({
  resolveGaussSceneUrl: (path?: string | null) => {
    const trimmed = path?.trim()
    if (!trimmed) return undefined
    return trimmed.startsWith('/') ? trimmed : `/${trimmed}`
  },
}))

vi.mock('../voice/composables/useVoicePlayer', () => ({
  useVoicePlayer: () => ({
    isSpeaking: ref(false),
    speakText: mockSpeakText,
    stopSpeaking: vi.fn(),
  }),
}))

vi.mock('../voice/composables/useVoiceInput', () => ({
  useVoiceInput: () => ({
    isSupported: true,
    ...mockVoiceState,
    setEnabled: mockVoiceSetEnabled,
  }),
}))

vi.mock('../voice/services/tts-service', () => ({
  ttsService: {
    setVoiceFileReader: vi.fn(),
  },
}))

vi.mock('../voice/services/voice-api', () => ({
  tauriVoiceFileReader: {},
}))

function makeMockPetConfig(): PetConfig {
  return {
    ...DEFAULT_PET_CONFIG,
    vrmExpressionEnabled: false,
    ttsEnabled: false,
    asrEnabled: false,
    asrLanguage: 'zh-CN',
    gaussSceneList: [
      { id: 'transparent', name: 'Transparent', path: '', isDefault: true },
      { id: 'home', name: 'Home', path: 'vrm/scene/home.spz', isDefault: true },
      { id: 'sea', name: 'Sea', path: 'vrm/scene/sea.spz', isDefault: true },
      { id: 'space', name: 'Space', path: 'vrm/scene/space.spz', isDefault: true },
    ],
    selectedGaussSceneId: 'transparent',
  } as PetConfig
}

async function setup(props: { isTyping?: boolean; messages?: PetMessage[] } = {}) {
  mockConfig.value = makeMockPetConfig()
  mockVoiceSetEnabled.mockClear()
  mockVoiceState.isEnabled.value = false
  mockVoiceState.isListening.value = false
  mockVoiceState.isProcessing.value = false
  mockVoiceState.error.value = null
  mockSpeakText.mockClear()
  mockGetDesktopPetEmotionSignal.mockReset()
  mockGetDesktopPetEmotionSignal.mockReturnValue(null)
  const { default: DivaPetView } = await import('./DivaPetView.vue')

  const wrapper = mount(DivaPetView, {
    props: {
      messages: props.messages ?? [],
      isTyping: props.isTyping ?? false,
      currentEmotion: undefined,
      desktopPetActive: false,
    },
  })

  await nextTick()
  await nextTick()

  return { wrapper }
}

const SCENE_BUTTON_SELECTOR = 'button[title="Switch Scene"]'

describe('DivaPetView scene picker', () => {
  beforeEach(() => {
    mockConfig.value = makeMockPetConfig()
  })

  it('shows the settings and scene buttons', async () => {
    const { wrapper } = await setup()

    expect(wrapper.find('.settings-icon').exists()).toBe(true)
    expect(wrapper.find('.image-icon').exists()).toBe(true)
    expect(wrapper.find(SCENE_BUTTON_SELECTOR).exists()).toBe(true)
  })

  it('shows a new-topic button in the whispers header', async () => {
    const { wrapper } = await setup()

    const button = wrapper.get('.pet-chat-new-topic-button')
    expect(button.attributes('title')).toBe('新建聊天')
    expect(wrapper.find('.plus-icon').exists()).toBe(true)
  })

  it('emits new-topic without directly invoking Diva TTS when clicking the whispers plus button', async () => {
    const { wrapper } = await setup()

    await wrapper.get('.pet-chat-new-topic-button').trigger('click')

    expect(wrapper.emitted('new-topic')?.[0]).toEqual(['让我们换个话题聊聊吧'])
    expect(mockSpeakText).not.toHaveBeenCalled()
  })

  it('disables the whispers new-topic button while typing', async () => {
    const { wrapper } = await setup({ isTyping: true })
    const button = wrapper.get('.pet-chat-new-topic-button')

    expect(button.attributes('disabled')).toBeDefined()
    await button.trigger('click')

    expect(wrapper.emitted('new-topic')).toBeUndefined()
    expect(mockSpeakText).not.toHaveBeenCalled()
  })

  it('shows only the streaming message thinking bubble while typing', async () => {
    const { wrapper } = await setup({
      isTyping: true,
      messages: [
        { role: 'user', content: 'hello', timestamp: 1 },
        { role: 'agent', content: '', isStreaming: true, timestamp: 2 },
      ],
    })

    const agentBubbles = wrapper.findAll('.pet-agent-bubble')

    expect(agentBubbles).toHaveLength(1)
    expect(agentBubbles[0].find('.loader-icon').exists()).toBe(true)
    expect(agentBubbles[0].text()).toContain('chat.thinking')
  })

  it('shows a push-to-talk button in the voice panel instead of the whispers header', async () => {
    const { wrapper } = await setup()

    const button = wrapper.get('.voice-panel-ptt-button')
    const panelButtons = wrapper.findAll('.voice-panel-stub button')

    expect(panelButtons[panelButtons.length - 1].element).toBe(button.element)
    expect(wrapper.find('.voice-panel-test-button').exists()).toBe(false)
    expect(button.text()).toContain('按住说话')
    expect(wrapper.find('.pet-header-ptt-button').exists()).toBe(false)
  })

  it('starts and stops voice input while the header push-to-talk button is held', async () => {
    const { wrapper } = await setup()
    const button = wrapper.get('.voice-panel-ptt-button')

    await button.trigger('pointerdown')
    expect(mockVoiceSetEnabled).toHaveBeenCalledWith(true)

    await button.trigger('pointerup')
    expect(mockVoiceSetEnabled).toHaveBeenLastCalledWith(false)
  })

  it('disables the header push-to-talk button while typing', async () => {
    const { wrapper } = await setup({ isTyping: true })
    const button = wrapper.get('.voice-panel-ptt-button')

    expect(button.attributes('disabled')).toBeDefined()
    await button.trigger('pointerdown')

    expect(mockVoiceSetEnabled).not.toHaveBeenCalled()
  })

  it('does not show the mood badge when the latest mood resolves to neutral', async () => {
    mockGetDesktopPetEmotionSignal.mockReturnValue(null)
    const { wrapper } = await setup({
      messages: [
        { role: 'agent', content: 'I am happy to help.', timestamp: 1 },
        { role: 'agent', content: 'Here is the answer.', timestamp: 2 },
      ],
    })

    expect(wrapper.find('[data-testid="pet-mood-badge"]').exists()).toBe(false)
  })

  it('shows the mood badge for a new non-neutral message and clears it after one second', async () => {
    vi.useFakeTimers()
    const messages: PetMessage[] = [{ role: 'agent', content: 'I am happy to help.', timestamp: 1 }]
    const { wrapper } = await setup({ messages: [] })
    mockGetDesktopPetEmotionSignal.mockReturnValue({ signature: '1:I am happy to help.', mood: 'happy' })
    await wrapper.setProps({ messages: [...messages] })
    await nextTick()

    let badge = wrapper.find('[data-testid="pet-mood-badge"]')
    expect(badge.exists()).toBe(true)
    expect(badge.text()).toContain('happy')

    vi.advanceTimersByTime(4000)
    await nextTick()

    badge = wrapper.find('[data-testid="pet-mood-badge"]')
    expect(badge.exists()).toBe(false)
    vi.useRealTimers()
  })

  it('does not replay a historical happy message on mount', async () => {
    mockGetDesktopPetEmotionSignal.mockReturnValue({ signature: '1:I am happy to help.', mood: 'happy' })
    const { wrapper } = await setup({
      messages: [{ role: 'agent', content: 'I am happy to help.', timestamp: 1 }],
    })

    expect(wrapper.find('[data-testid="pet-mood-badge"]').exists()).toBe(false)
  })

  it('opens the scene menu with all configured scenes', async () => {
    const { wrapper } = await setup()
    const sceneButton = wrapper.get(SCENE_BUTTON_SELECTOR)

    await sceneButton.trigger('click')
    await nextTick()
    await nextTick()

    const text = wrapper.text()
    expect(text).toContain('Transparent')
    expect(text).toContain('Home')
    expect(text).toContain('Sea')
    expect(text).toContain('Space')
  })

  it('highlights the active scene', async () => {
    const { wrapper } = await setup()
    mockConfig.value.selectedGaussSceneId = 'sea'
    await nextTick()
    const sceneButton = wrapper.get(SCENE_BUTTON_SELECTOR)

    await sceneButton.trigger('click')
    await nextTick()
    await nextTick()

    const items = wrapper.findAll('[class*="cursor-pointer"]')
    const seaItem = items.find((el) => el.text().includes('Sea'))
    const transparentItem = items.find((el) => el.text().includes('Transparent'))

    expect(seaItem?.attributes('class')).toContain('text-cyan-100')
    expect(transparentItem?.attributes('class')).not.toContain('text-cyan-100')
  })

  it('updates the selected scene when an item is clicked', async () => {
    const { wrapper } = await setup()
    const sceneButton = wrapper.get(SCENE_BUTTON_SELECTOR)

    await sceneButton.trigger('click')
    await nextTick()
    await nextTick()

    const items = wrapper.findAll('[class*="cursor-pointer"]')
    const spaceItem = items.find((el) => el.text().includes('Space'))

    expect(mockConfig.value.selectedGaussSceneId).toBe('transparent')
    await spaceItem!.trigger('click')
    await nextTick()
    await nextTick()
    expect(mockConfig.value.selectedGaussSceneId).toBe('space')
  })

  it('closes the scene menu when clicking the outside overlay', async () => {
    const { wrapper } = await setup()
    const sceneButton = wrapper.get(SCENE_BUTTON_SELECTOR)

    await sceneButton.trigger('click')
    await nextTick()
    await nextTick()

    const overlay = wrapper.get('.fixed.inset-0')
    await overlay.trigger('click')
    await nextTick()
    await nextTick()

    const remainingItems = wrapper.findAll('[class*="cursor-pointer"]')
    expect(remainingItems.filter((el) => el.text().includes('Transparent')).length).toBe(0)
  })

  it('passes resolved model and scene props into the isolated embedded host', async () => {
    const { wrapper } = await setup()
    mockConfig.value.selectedGaussSceneId = 'sea'
    await nextTick()

    const embeddedHost = wrapper.getComponent({ name: 'EmbeddedPetFrame' })
    expect(embeddedHost.props('modelPath')).toBe('/mock/path/model.vrm')
    expect(embeddedHost.props('backgroundScene')).toBe('sea')
    expect(embeddedHost.props('backgroundSceneUrl')).toBe('/vrm/scene/sea.spz')
    expect(embeddedHost.props('transparentBackground')).toBe(false)
    expect(embeddedHost.props('active')).toBe(true)
  })

  it('passes visual transparency without loading a runtime scene for the transparent scene', async () => {
    const { wrapper } = await setup()

    const embeddedHost = wrapper.getComponent({ name: 'EmbeddedPetFrame' })
    expect(embeddedHost.props('backgroundScene')).toBeUndefined()
    expect(embeddedHost.props('backgroundSceneUrl')).toBeUndefined()
    expect(embeddedHost.props('transparentBackground')).toBe(true)
  })
})
