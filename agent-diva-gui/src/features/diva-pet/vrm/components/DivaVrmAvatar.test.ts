import { describe, expect, it, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'

const { mockSetBackgroundScene, mockCreateRuntime, runtimeCallOrder } = vi.hoisted(() => {
  const runtimeCallOrder: string[] = []
  const mockSetBackgroundScene = vi.fn<(...args: unknown[]) => Promise<void>>()

  const mockCreateRuntime = vi.fn<any>(async () => ({
    setBackgroundScene: vi.fn((...args: unknown[]) => {
      runtimeCallOrder.push('setBackgroundScene')
      return mockSetBackgroundScene(...args)
    }),
    loadCharacter: vi.fn(() => {
      runtimeCallOrder.push('loadCharacter')
      return Promise.resolve(undefined)
    }),
    setMood: vi.fn(),
    setSpeechState: vi.fn().mockResolvedValue(undefined),
    resume: vi.fn(),
    pause: vi.fn(),
    resize: vi.fn(() => {
      runtimeCallOrder.push('resize')
    }),
    setTransform: vi.fn().mockResolvedValue(undefined),
    destroy: vi.fn().mockResolvedValue(undefined),
    bridge: {
      on: vi.fn(() => vi.fn()),
    },
  }))

  return { mockSetBackgroundScene, mockCreateRuntime, runtimeCallOrder }
})

class MockResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver = MockResizeObserver as unknown as typeof ResizeObserver

vi.mock('avatar-runtime-vrm', () => ({
  createVrmRuntime: mockCreateRuntime,
}))

async function setup(props: Record<string, unknown> = {}) {
  vi.clearAllMocks()
  runtimeCallOrder.length = 0
  mockSetBackgroundScene.mockResolvedValue(undefined)
  mockCreateRuntime.mockClear()

  const { default: DivaVrmAvatar } = await import('./DivaVrmAvatar.vue')
  const wrapper = mount(DivaVrmAvatar, {
    props: {
      modelPath: '/test.vrm',
      ...props,
    },
  })

  await vi.waitFor(
    () => {
      expect(mockCreateRuntime.mock.calls.length).toBeGreaterThanOrEqual(1)
    },
    { timeout: 1000, interval: 5 },
  )
  for (let i = 0; i < 20; i++) {
    await nextTick()
  }

  return { wrapper }
}

describe('background scene integration', () => {
  beforeEach(() => {
    mockSetBackgroundScene.mockResolvedValue(undefined)
  })

  it('passes scene id and url to runtime.setBackgroundScene', async () => {
    await setup({ backgroundScene: 'home', backgroundSceneUrl: '/vrm/scene/home.spz' })

    expect(mockSetBackgroundScene).toHaveBeenCalledWith('home', '/vrm/scene/home.spz')
    expect(mockSetBackgroundScene).toHaveBeenCalledTimes(1)
  })

  it('keeps transparent scene loading separate from model loading', async () => {
    await setup({ backgroundScene: 'transparent' })

    expect(mockSetBackgroundScene).toHaveBeenCalledWith('transparent', undefined)
    expect(mockSetBackgroundScene).toHaveBeenCalledTimes(1)
  })

  it('switches scenes with the selected scene url', async () => {
    const { wrapper } = await setup({
      backgroundScene: 'home',
      backgroundSceneUrl: '/vrm/scene/home.spz',
    })
    mockSetBackgroundScene.mockClear()

    await wrapper.setProps({
      backgroundScene: 'sea',
      backgroundSceneUrl: '/vrm/scene/sea.spz',
    })
    await nextTick()
    await nextTick()
    await nextTick()

    expect(mockSetBackgroundScene).toHaveBeenCalledWith('sea', '/vrm/scene/sea.spz')
    expect(mockSetBackgroundScene).toHaveBeenCalledTimes(1)
  })

  it('resyncs when backgroundSceneUrl changes after mount', async () => {
    const { wrapper } = await setup({ backgroundScene: 'home' })
    mockSetBackgroundScene.mockClear()

    await wrapper.setProps({ backgroundSceneUrl: '/vrm/scene/home.spz' })
    await nextTick()
    await nextTick()
    await nextTick()

    expect(mockSetBackgroundScene).toHaveBeenCalledWith('home', '/vrm/scene/home.spz')
    expect(mockSetBackgroundScene).toHaveBeenCalledTimes(1)
  })

  it('falls back to transparent when scene loading fails', async () => {
    mockSetBackgroundScene.mockRejectedValueOnce(new Error('Scene file not found'))

    await setup({ backgroundScene: 'home', backgroundSceneUrl: '/vrm/scene/home.spz' })

    expect(mockSetBackgroundScene).toHaveBeenCalledWith('home', '/vrm/scene/home.spz')
    expect(mockSetBackgroundScene).toHaveBeenCalledWith('transparent')
  })

  it('handles rapid scene switching', async () => {
    const { wrapper } = await setup({
      backgroundScene: 'home',
      backgroundSceneUrl: '/vrm/scene/home.spz',
    })
    mockSetBackgroundScene.mockClear()

    await wrapper.setProps({ backgroundScene: 'sea', backgroundSceneUrl: '/vrm/scene/sea.spz' })
    await nextTick()
    await wrapper.setProps({ backgroundScene: 'space', backgroundSceneUrl: '/vrm/scene/space.spz' })
    await nextTick()
    await wrapper.setProps({ backgroundScene: 'transparent', backgroundSceneUrl: undefined })
    await nextTick()
    await nextTick()
    await nextTick()

    expect(mockSetBackgroundScene).toHaveBeenCalledWith('sea', '/vrm/scene/sea.spz')
    expect(mockSetBackgroundScene).toHaveBeenCalledWith('space', '/vrm/scene/space.spz')
    expect(mockSetBackgroundScene).toHaveBeenCalledWith('transparent', undefined)

    const lastCallArgs = mockSetBackgroundScene.mock.calls[mockSetBackgroundScene.mock.calls.length - 1]
    expect(lastCallArgs[0]).toBe('transparent')
  })

  it('applies scene changes after mount', async () => {
    const { wrapper } = await setup()
    mockSetBackgroundScene.mockClear()

    await wrapper.setProps({ backgroundScene: 'sea', backgroundSceneUrl: '/vrm/scene/sea.spz' })
    await nextTick()
    await nextTick()
    await nextTick()

    expect(mockSetBackgroundScene).toHaveBeenCalledWith('sea', '/vrm/scene/sea.spz')
    expect(mockSetBackgroundScene).toHaveBeenCalledTimes(1)
  })

  it('supports scene loading in desktopPet mode', async () => {
    await setup({
      backgroundScene: 'home',
      backgroundSceneUrl: '/vrm/scene/home.spz',
      desktopPet: true,
    })

    expect(mockSetBackgroundScene).toHaveBeenCalledWith('home', '/vrm/scene/home.spz')
  })
})

describe('runtime initialization', () => {
  it('uses embedded runtime options by default', async () => {
    await setup()

    const options = mockCreateRuntime.mock.lastCall?.[1] as Record<string, unknown> | undefined
    expect(options?.mode).toBe('embedded')
    expect(options?.transparent).toBe(false)
    expect(options?.allowInteraction).toBe(true)
    expect(options?.backgroundColor).toBe('#ffffff')
    expect(options?.maxFps).toBe(60)
  })

  it('uses transparent desktop-pet runtime options in desktopPet mode', async () => {
    await setup({ desktopPet: true })

    const options = mockCreateRuntime.mock.lastCall?.[1] as Record<string, unknown> | undefined
    expect(options?.mode).toBe('desktop-pet')
    expect(options?.transparent).toBe(true)
    expect(options?.allowInteraction).toBe(true)
    expect(options?.backgroundColor).toBeNull()
    expect(options?.maxFps).toBe(24)
  })
})

describe('runtime ordering', () => {
  it('calls resize before loadCharacter', async () => {
    await setup()

    const resizeIdx = runtimeCallOrder.indexOf('resize')
    const loadCharIdx = runtimeCallOrder.indexOf('loadCharacter')

    expect(resizeIdx).toBeGreaterThanOrEqual(0)
    expect(loadCharIdx).toBeGreaterThanOrEqual(0)
    expect(resizeIdx).toBeLessThan(loadCharIdx)
  })

  it('calls resize before setBackgroundScene', async () => {
    await setup({ backgroundScene: 'home', backgroundSceneUrl: '/vrm/scene/home.spz' })

    const resizeIdx = runtimeCallOrder.indexOf('resize')
    const setBgIdx = runtimeCallOrder.indexOf('setBackgroundScene')

    expect(resizeIdx).toBeGreaterThanOrEqual(0)
    expect(setBgIdx).toBeGreaterThanOrEqual(0)
    expect(resizeIdx).toBeLessThan(setBgIdx)
  })
})
