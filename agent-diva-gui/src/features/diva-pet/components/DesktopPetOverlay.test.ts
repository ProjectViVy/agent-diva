import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { ref, nextTick } from 'vue'
import type { PetConfig } from '../types'
import { DEFAULT_PET_CONFIG } from '../types'

// ═══════════════════════════════════════════════════════════════════
//  Hoisted mocks — must be defined before vi.mock (hoisted) references them
// ═══════════════════════════════════════════════════════════════════

const { mockInvoke, mockGetCurrentWindowFn, mockListen, mockEmitTo, mockVoiceSetEnabled, mockVoiceInputOptions, mockVoiceState } = vi.hoisted(() => {
  const mockVoiceState = {
    error: { value: null as string | null },
    isEnabled: { value: false },
    isListening: { value: false },
    isProcessing: { value: false },
  }
  return {
    mockInvoke: vi.fn<(...args: unknown[]) => Promise<unknown>>(),
    mockGetCurrentWindowFn: vi.fn(() => ({
      startDragging: vi.fn(() => Promise.resolve()),
      minimize: vi.fn(() => Promise.resolve()),
    })),
    mockListen: vi.fn<(...args: unknown[]) => Promise<() => void>>(() => Promise.resolve(() => {})),
    mockEmitTo: vi.fn(() => Promise.resolve(undefined)),
    mockVoiceSetEnabled: vi.fn((enabled: boolean) => {
      mockVoiceState.isEnabled.value = enabled
      return Promise.resolve(true)
    }),
    mockVoiceInputOptions: { current: null as null | { onRecognizedText: (text: string) => Promise<void> | void } },
    mockVoiceState,
  }
})

// ── Tauri API mocks ──────────────────────────────────────────────

vi.mock('@tauri-apps/api/core', () => ({ invoke: mockInvoke }))
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: mockGetCurrentWindowFn }))
vi.mock('@tauri-apps/api/event', () => ({ listen: mockListen, emitTo: mockEmitTo }))

vi.mock('../voice/composables/useVoiceInput', () => ({
  useVoiceInput: (options: { onRecognizedText: (text: string) => Promise<void> | void }) => {
    mockVoiceInputOptions.current = options
    return {
      ...mockVoiceState,
      isSupported: true,
      note: { value: null },
      pauseFor: vi.fn(),
      setEnabled: mockVoiceSetEnabled,
      toggle: vi.fn(),
    }
  },
}))

// ── DivaVrmAvatar stub (avoids Three.js import in test env) ─────

vi.mock('../vrm/components/DivaVrmAvatar.vue', () => ({
  default: {
    name: 'DivaVrmAvatar',
    template: '<div class="diva-vrm-avatar-stub" />',
    props: ['modelPath', 'mood', 'isSpeaking', 'desktopPet', 'active', 'backgroundScene', 'backgroundSceneUrl', 'idleMotionEnabled', 'selectedMotionIds', 'startMotionId', 'startMotionToken'],
    emits: [],
    setup() {
      return { setScale: vi.fn(), playMotion: vi.fn(), stopMotion: vi.fn(), getScale: vi.fn(() => 1.0) }
    },
  },
}))

// ── Pet-config mock with reactive config ─────────────────────────

function makeMockPetConfig(): PetConfig {
  return {
    ...DEFAULT_PET_CONFIG,
    desktopPetScale: 1.0,
    desktopPetAlwaysOnTop: true,
    subtitleEnabled: true,
    vrmMotionEnabled: false,
    vrmExpressionEnabled: false,
    ttsEnabled: false,
    asrEnabled: false,
    vrmAppearances: [
      {
        id: 'alt',
        name: '备用外观',
        modelId: 'test2',
        motionIds: ['idle'],
        startMotionId: 'greeting',
        expressionEnabled: true,
        motionEnabled: true,
      },
    ],
    activeAppearanceId: 'default',
  } as PetConfig
}

const mockConfig = ref<PetConfig>(makeMockPetConfig())

vi.mock('../services/pet-config', () => ({
  usePetConfig: () => ({
    config: mockConfig,
    setEnabled: vi.fn(),
    updateConfig: vi.fn((patch: Partial<PetConfig>) => {
      mockConfig.value = { ...mockConfig.value, ...patch }
    }),
  }),
}))

// ── Utility mocks ────────────────────────────────────────────────

vi.mock('../utils/mood', () => ({
  normalizeMood: vi.fn((payload: string) => payload),
}))

vi.mock('../utils/vrm-model', () => ({
  resolveVrmModelPath: vi.fn(() => '/mock/path/model.vrm'),
}))

// ── Subtitle overlay mock ────────────────────────────────────────

const mockSubtitle = ref({
  visible: false,
  text: '',
  position: { x: 100, y: 100 },
  isDragging: false,
})

vi.mock('./subtitle-overlay', () => ({
  useSubtitleOverlay: () => ({
    subtitle: mockSubtitle,
    init: vi.fn(),
    startDrag: vi.fn(),
    onDrag: vi.fn(),
    endDrag: vi.fn(),
  }),
}))

// ── Icon library stub ────────────────────────────────────────────

vi.mock('lucide-vue-next', () => ({
  Mic: {
    name: 'Mic',
    template: '<span class="mic-icon"/>',
  },
}))

// ═══════════════════════════════════════════════════════════════════
//  Test setup helper
// ═══════════════════════════════════════════════════════════════════

/** Reset all mocks and config, then mount component. */
async function setup() {
  const registeredListeners = new Map<string, (event: { payload: unknown }) => void>()

  // Reset mock call history
  mockInvoke.mockReset()
  mockListen.mockReset()
  mockEmitTo.mockReset()
  mockVoiceSetEnabled.mockClear()
  mockVoiceState.error.value = null
  mockVoiceState.isEnabled.value = false
  mockVoiceState.isListening.value = false
  mockVoiceState.isProcessing.value = false
  mockVoiceInputOptions.current = null
  mockGetCurrentWindowFn.mockReset()
  mockGetCurrentWindowFn.mockReturnValue({
    startDragging: vi.fn(() => Promise.resolve()),
    minimize: vi.fn(() => Promise.resolve()),
  })

  // Reset config to initial state
  mockConfig.value = makeMockPetConfig()

  // Default: all invoke calls succeed
  mockInvoke.mockResolvedValue(undefined)
  mockListen.mockImplementation(async (...args: unknown[]) => {
    const [event, callback] = args as [string, (payload: { payload: unknown }) => void]
    registeredListeners.set(event, callback)
    return () => {}
  })

  // Ensure navigator.mediaDevices exists (happy-dom may lack it)
  if (!('mediaDevices' in navigator)) {
    Object.defineProperty(navigator, 'mediaDevices', {
      value: {
        getUserMedia: vi.fn(() => Promise.reject(new Error('not implemented'))),
      },
      configurable: true,
      writable: true,
    })
  }

  // Dynamic import ensures vi.mock hoisting is applied
  const { default: DesktopPetOverlay } = await import(
    './DesktopPetOverlay.vue'
  )

  const wrapper = mount(DesktopPetOverlay, {
    global: {
      stubs: {
        // Teleport/transition stubs if needed; Vue test-utils handles
        // <Transition> by rendering its content synchronously.
      },
    },
  })

  // Let onMounted promises (listen calls, etc.) settle
  await nextTick()
  await nextTick()

  return { wrapper, registeredListeners }
}

/** Open the context menu at given coordinates. */
async function openMenu(
  wrapper: ReturnType<typeof mount>,
  x = 200,
  y = 300,
) {
  await wrapper.find('.desktop-pet-overlay').trigger('contextmenu', {
    clientX: x,
    clientY: y,
  })
  await nextTick()
  await nextTick()
}

// ═══════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════

describe('DesktopPetOverlay', () => {
  beforeEach(() => {
    mockInvoke.mockResolvedValue(undefined)
    mockListen.mockResolvedValue(() => {})
  })

  it('forces transparent background in desktop-pet mode', async () => {
    mockConfig.value.selectedGaussSceneId = 'sea'
    const { wrapper } = await setup()

    expect(wrapper.find('.desktop-pet-overlay').classes()).not.toContain('desktop-pet-app-background')

    const avatar = wrapper.findComponent({ name: 'DivaVrmAvatar' })
    expect(avatar.props('backgroundScene')).toBe('transparent')
    expect(avatar.props('backgroundSceneUrl')).toBeUndefined()
  })

  it('shows the push-to-talk button and toggles voice input while held', async () => {
    const { wrapper } = await setup()
    mockConfig.value.asrEnabled = true
    await nextTick()

    const button = wrapper.get('.ptt-btn')

    await button.trigger('pointerdown')
    expect(mockVoiceSetEnabled).toHaveBeenCalledWith(true)

    await button.trigger('pointerup')
    expect(mockVoiceSetEnabled).toHaveBeenLastCalledWith(false)
  })

  it('stops voice input when pointer leaves the push-to-talk button', async () => {
    const { wrapper } = await setup()
    mockConfig.value.asrEnabled = true
    await nextTick()
    const button = wrapper.get('.ptt-btn')

    await button.trigger('pointerdown')
    await button.trigger('pointerleave')

    expect(mockVoiceSetEnabled).toHaveBeenNthCalledWith(1, true)
    expect(mockVoiceSetEnabled).toHaveBeenNthCalledWith(2, false)
  })

  it('emits recognized desktop-pet voice text to the main window', async () => {
    await setup()

    await mockVoiceInputOptions.current?.onRecognizedText('  hello diva  ')

    expect(mockEmitTo).toHaveBeenCalledWith('main', 'desktop-pet-voice-message', 'hello diva')
  })

  it('flashes a received mood for one second and then resets to neutral', async () => {
    vi.useFakeTimers()
    const { wrapper, registeredListeners } = await setup()
    mockConfig.value.vrmExpressionEnabled = true
    await nextTick()

    registeredListeners.get('desktop-pet-emotion')?.({ payload: 'happy' })
    await nextTick()

    const avatar = wrapper.getComponent({ name: 'DivaVrmAvatar' })
    expect(avatar.props('mood')).toBe('happy')

    vi.advanceTimersByTime(4000)
    await nextTick()

    expect(avatar.props('mood')).toBe('neutral')
    vi.useRealTimers()
  })

  it('keeps the desktop pet neutral when expression mapping is disabled', async () => {
    const { wrapper, registeredListeners } = await setup()
    mockConfig.value.vrmExpressionEnabled = false
    await nextTick()

    registeredListeners.get('desktop-pet-emotion')?.({ payload: 'happy' })
    await nextTick()

    const avatar = wrapper.getComponent({ name: 'DivaVrmAvatar' })
    expect(avatar.props('mood')).toBe('neutral')
  })

  // ── 1. Right-click context menu ───────────────────────────────

  describe('right-click context menu', () => {
    it('shows context menu on right-click', async () => {
      const { wrapper } = await setup()

      // Initially no menu
      expect(wrapper.find('.context-menu').exists()).toBe(false)

      // Right-click overlay
      await openMenu(wrapper)

      // Menu should appear
      expect(wrapper.find('.context-menu').exists()).toBe(true)
    })
  })

  // ── 2. Click on overlay hides menu ────────────────────────────

  describe('click hides context menu', () => {
    it('hides context menu on overlay click', async () => {
      const { wrapper } = await setup()

      // Show menu first
      await openMenu(wrapper)
      expect(wrapper.find('.context-menu').exists()).toBe(true)

      // Click on overlay (not on menu — overlay's @click fires)
      await wrapper.find('.desktop-pet-overlay').trigger('click')
      await nextTick()
      await nextTick()

      expect(wrapper.find('.context-menu').exists()).toBe(false)
    })
  })

  // ── 3. Submenu hover expands ──────────────────────────────────

  describe('submenu hover expands', () => {
    it('sets activeSubmenu on mouseenter of appearance menu item', async () => {
      const { wrapper } = await setup()
      await openMenu(wrapper)

      // Hover the first menu-item-has-sub (🎭 切换外观)
      const appearanceItem = wrapper.find('.menu-item-has-sub')
      await appearanceItem.trigger('mouseenter')
      await nextTick()

      // The submenu should now be visible (v-if="activeSubmenu === 'appearance'")
      const submenu = wrapper.find('.submenu')
      expect(submenu.exists()).toBe(true)

      const items = submenu.findAll('.submenu-item')
      expect(items.some((el) => el.text().includes('默认角色'))).toBe(true)
      expect(items.some((el) => el.text().includes('备用外观'))).toBe(true)
    })

    it('uses the built-in default appearance when user appearances are empty', async () => {
      const { wrapper } = await setup()
      mockConfig.value.vrmAppearances = []
      mockConfig.value.activeAppearanceId = 'missing'
      await nextTick()
      await openMenu(wrapper)

      const appearanceItem = wrapper.find('.menu-item-has-sub')
      await appearanceItem.trigger('mouseenter')
      await nextTick()

      const submenuItems = wrapper.find('.submenu').findAll('.submenu-item')
      expect(submenuItems).toHaveLength(1)
      expect(submenuItems[0].text()).toContain('默认角色')
      expect(mockConfig.value.activeAppearanceId).toBe('default')
      expect(mockConfig.value.vrmModel).toBe('/vrm/models/Alice.vrm')
    })

    it('applies model, motions, motion switch, and expression switch when changing appearance', async () => {
      const { wrapper } = await setup()
      await openMenu(wrapper)

      const appearanceItem = wrapper.find('.menu-item-has-sub')
      await appearanceItem.trigger('mouseenter')
      await nextTick()

      const altItem = wrapper.findAll('.submenu-item').find((item) => item.text().includes('备用外观'))
      expect(altItem).toBeTruthy()
      await altItem!.trigger('click')
      await nextTick()

      expect(mockConfig.value.activeAppearanceId).toBe('alt')
      expect(mockConfig.value.vrmModel).toBe('test2')
      expect(mockConfig.value.selectedMotionIds).toEqual(['idle'])
      expect(mockConfig.value.vrmMotionEnabled).toBe(true)
      expect(mockConfig.value.vrmExpressionEnabled).toBe(true)

      const avatar = wrapper.getComponent({ name: 'DivaVrmAvatar' })
      expect(avatar.props('startMotionId')).toBe('greeting')
      expect(avatar.props('startMotionToken')).toBe('alt')
    })
  })

  // ── 4. Submenu mouseleave doesn't immediately close ───────────

  describe('submenu mouseleave no immediate close', () => {
    it('keeps submenu visible after mouseleave (closeSubmenu is no-op)', async () => {
      const { wrapper } = await setup()
      await openMenu(wrapper)

      const appearanceItem = wrapper.find('.menu-item-has-sub')

      // Open submenu
      await appearanceItem.trigger('mouseenter')
      await nextTick()
      expect(wrapper.find('.submenu').exists()).toBe(true)

      // Mouseleave — closeSubmenu is a no-op
      await appearanceItem.trigger('mouseleave')
      await nextTick()

      // Submenu should STILL be visible because closeSubmenu does nothing
      expect(wrapper.find('.submenu').exists()).toBe(true)
    })
  })

  // ── 5. Toggle pass-through ────────────────────────────────────

  describe('toggle pass-through', () => {
    it('calls invoke with set_desktop_pet_ignore_mouse on toggle', async () => {
      const { wrapper } = await setup()
      await openMenu(wrapper)

      // First toggle item is pass-through (🖱️ 穿透切换)
      const toggles = wrapper.findAll('.menu-item-toggle')
      const passThroughItem = toggles[0]
      expect(passThroughItem.text()).toContain('穿透')

      await passThroughItem.trigger('click')
      await nextTick()
      await nextTick()

      expect(mockInvoke).toHaveBeenCalledWith(
        'set_desktop_pet_ignore_mouse',
        expect.objectContaining({ ignore: true }),
      )
    })
  })

  // ── 6. Toggle always-on-top ───────────────────────────────────

  describe('toggle always-on-top', () => {
    it('calls invoke with set_desktop_pet_always_on_top', async () => {
      const { wrapper } = await setup()
      await openMenu(wrapper)

      // Second toggle item is always-on-top (📌 窗口置顶)
      const toggles = wrapper.findAll('.menu-item-toggle')
      const alwaysOnTopItem = toggles[1]
      expect(alwaysOnTopItem.text()).toContain('置顶')

      // Default is ON (isAlwaysOnTop = true)
      expect(alwaysOnTopItem.find('.menu-toggle-state.on').exists()).toBe(true)

      await alwaysOnTopItem.trigger('click')
      await nextTick()
      await nextTick()

      expect(mockInvoke).toHaveBeenCalledWith(
        'set_desktop_pet_always_on_top',
        expect.objectContaining({ alwaysOnTop: false }),
      )
    })
  })

  // ── 7. Toggle always-on-top failure rolls back ────────────────

  describe('toggle always-on-top failure', () => {
    it('rolls back isAlwaysOnTop state on invoke failure', async () => {
      const { wrapper } = await setup()

      // Make invoke fail AFTER setup (setup() resets mocks)
      mockInvoke.mockRejectedValueOnce(new Error('Tauri API unavailable'))

      await openMenu(wrapper)

      const toggles = wrapper.findAll('.menu-item-toggle')
      const alwaysOnTopItem = toggles[1]

      // Initial state: ON
      expect(alwaysOnTopItem.find('.menu-toggle-state.on').exists()).toBe(true)

      await alwaysOnTopItem.trigger('click')
      await nextTick()
      await nextTick()

      // After failure: still ON (rolled back to previous = true)
      expect(alwaysOnTopItem.find('.menu-toggle-state.on').exists()).toBe(true)
      expect(mockConfig.value.desktopPetAlwaysOnTop).toBe(true)
    })
  })

  // ── 8. Scale slider input ─────────────────────────────────────

  describe('scale slider', () => {
    it('updates desktopPetScale and persists to config on input', async () => {
      const { wrapper } = await setup()
      await openMenu(wrapper)

      const slider = wrapper.find('.menu-slider')
      expect(slider.exists()).toBe(true)
      expect((slider.element as HTMLInputElement).value).toBe('1')

      // Simulate input change to 1.25
      await slider.setValue('1.25')
      await nextTick()
      await nextTick()

      // Config should reflect new value
      expect(mockConfig.value.desktopPetScale).toBe(1.25)
    })
  })

  // ── 9. Close pet ──────────────────────────────────────────────

  describe('close pet', () => {
    it('calls invoke with close_desktop_pet', async () => {
      const { wrapper } = await setup()
      await openMenu(wrapper)

      // The danger menu item is close (❌ 关闭)
      const dangerItem = wrapper.find('.menu-item-danger')
      expect(dangerItem.text()).toContain('关闭')

      await dangerItem.trigger('click')
      await nextTick()
      await nextTick()

      expect(mockInvoke).toHaveBeenCalledWith('close_desktop_pet')
    })
  })

  // ── 10. Minimize pet ──────────────────────────────────────────

  describe('minimize pet', () => {
    it('calls invoke with minimize_desktop_pet', async () => {
      const { wrapper } = await setup()
      await openMenu(wrapper)

      // Find minimize item (💤 最小化)
      const allItems = wrapper.findAll('.menu-item')
      const minimizeItem = allItems.find(
        (item) => item.text().includes('最小化'),
      )
      expect(minimizeItem).toBeTruthy()

      await minimizeItem!.trigger('click')
      await nextTick()
      await nextTick()

      expect(mockInvoke).toHaveBeenCalledWith('minimize_desktop_pet')
    })
  })

  // ── 11. Subtitle toggle ───────────────────────────────────────

  describe('subtitle toggle', () => {
    it('flips subtitleEnabled in config when toggled in voice submenu', async () => {
      const { wrapper } = await setup()
      await openMenu(wrapper)

      // Open the voice submenu (🔊 语音设置 — 3rd submenu)
      const submenuTriggers = wrapper.findAll('.menu-item-has-sub')
      const voiceItem = submenuTriggers[2]
      expect(voiceItem.text()).toContain('语音')
      await voiceItem.trigger('mouseenter')
      await nextTick()

      // Find subtitle toggle inside the voice submenu
      const submenu = wrapper.find('.submenu')
      const toggleItems = submenu.findAll('.submenu-item.submenu-toggle')
      const subtitleToggle = toggleItems[2] // 字幕显示
      expect(subtitleToggle.text()).toContain('字幕')

      // Initially enabled
      expect(mockConfig.value.subtitleEnabled).toBe(true)

      await subtitleToggle.trigger('click')
      await nextTick()
      await nextTick()

      expect(mockConfig.value.subtitleEnabled).toBe(false)
    })
  })

  // ── 12. Auto-hide timer resets on menu mousemove ─────────────

  describe('auto-hide timer', () => {
    it('hides menu after MENU_AUTO_HIDE_MS of inactivity', async () => {
      vi.useFakeTimers()
      const { wrapper } = await setup()
      // flush any microtasks that fake timers didn't advance
      vi.advanceTimersByTime(100)
      await nextTick()

      await openMenu(wrapper)
      const menu = wrapper.find('.context-menu')
      expect(menu.exists()).toBe(true)

      // Advance past auto-hide timeout
      vi.advanceTimersByTime(3100)
      await nextTick()

      // Menu should be hidden
      expect(wrapper.find('.context-menu').exists()).toBe(false)

      vi.useRealTimers()
    })

    it('resets auto-hide timer on menu mousemove', async () => {
      vi.useFakeTimers()
      const { wrapper } = await setup()
      vi.advanceTimersByTime(100)
      await nextTick()

      await openMenu(wrapper)
      const menu = wrapper.find('.context-menu')
      expect(menu.exists()).toBe(true)

      // Advance to just before timeout
      vi.advanceTimersByTime(2500)
      await nextTick()
      expect(wrapper.find('.context-menu').exists()).toBe(true)

      // Mousemove resets the timer
      await menu.trigger('mousemove')
      await nextTick()

      // Advance another 2500ms — still within new timeout window
      vi.advanceTimersByTime(2500)
      await nextTick()
      expect(wrapper.find('.context-menu').exists()).toBe(true)

      // Advance past the remaining time
      vi.advanceTimersByTime(600)
      await nextTick()
      expect(wrapper.find('.context-menu').exists()).toBe(false)

      vi.useRealTimers()
    })
  })
})
