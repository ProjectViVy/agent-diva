import { beforeEach, describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick, ref } from 'vue'
import type { PetConfig } from '../types'
import { DEFAULT_PET_CONFIG } from '../types'

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
    ],
  },
}))

vi.mock('../voice/components/DivaPetVoicePanel.vue', () => ({
  default: {
    name: 'DivaPetVoicePanel',
    template: '<div class="voice-panel-stub" />',
    props: [
      'isSpeaking',
      'isVoiceSupported',
      'isVoiceEnabled',
      'isListening',
      'isProcessing',
      'voiceError',
      'ttsEnabled',
    ],
    emits: ['toggleVoice', 'update:ttsEnabled', 'testSpeak', 'stopSpeaking'],
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
}))

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}))

vi.mock('../utils/mood', () => ({
  deriveMoodFromMessages: vi.fn(() => 'neutral'),
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
    speakText: vi.fn(),
    stopSpeaking: vi.fn(),
  }),
}))

vi.mock('../voice/composables/useVoiceInput', () => ({
  useVoiceInput: () => ({
    isSupported: true,
    isEnabled: ref(false),
    isListening: ref(false),
    isProcessing: ref(false),
    error: ref(null),
    setEnabled: vi.fn(() => Promise.resolve(true)),
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

async function setup() {
  mockConfig.value = makeMockPetConfig()
  const { default: DivaPetView } = await import('./DivaPetView.vue')

  const wrapper = mount(DivaPetView, {
    props: {
      messages: [],
      isTyping: false,
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
    expect(embeddedHost.props('active')).toBe(true)
  })
})
