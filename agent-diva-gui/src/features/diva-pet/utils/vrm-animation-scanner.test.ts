import { afterEach, describe, expect, it, vi } from 'vitest'
import { buildKnownMotionInfo, scanVRMAnimations, VRM_ANIMATIONS_DIR } from '../utils/vrm-animation-scanner'

describe('vrm-animation-scanner', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('builds the runtime playable motion catalog', () => {
    const list = buildKnownMotionInfo()
    expect(list).toHaveLength(22)
    expect(list.find((motion) => motion.id === 'akimbo')?.kind).toBe('idle')
    expect(list.find((motion) => motion.id === 'LookAround')?.kind).toBe('idle')
    expect(list.find((motion) => motion.id === 'greeting')?.kind).toBe('oneshot')
    expect(list.find((motion) => motion.id === 'greeting')?.name).toBe('问候')
    expect(list.find((motion) => motion.id === 'Clapping')?.name).toBe('鼓掌')
    expect(list.find((motion) => motion.id === 'greeting')?.path).toBe(`${VRM_ANIMATIONS_DIR}/greeting.vrma`)
  })

  it('scans from manifest.json when available', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ animations: ['greeting.vrma', 'stretch.vrma'] }),
    } as Response)

    const result = await scanVRMAnimations()
    expect(result.map((motion) => motion.id)).toEqual(['greeting', 'stretch'])
    expect(result[0].kind).toBe('oneshot')
  })

  it('falls back to known animations and filters missing files', async () => {
    vi.spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce({ ok: false } as Response)
      .mockImplementation((url: string | URL | Request) => {
        const urlStr = typeof url === 'string' ? url : url instanceof URL ? url.toString() : url.url
        return Promise.resolve({
          ok: urlStr.includes('akimbo.vrma') || urlStr.includes('greeting.vrma'),
        } as Response)
      })

    const result = await scanVRMAnimations()
    expect(result.map((motion) => motion.id).sort()).toEqual(['akimbo', 'greeting'])
  })

  it('returns an empty list when all fetches fail', async () => {
    vi.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('Network error'))

    await expect(scanVRMAnimations()).resolves.toEqual([])
  })
})
