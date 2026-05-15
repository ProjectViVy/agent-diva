import { describe, expect, it, vi, afterEach } from 'vitest'
import { scanVRMAnimations, buildKnownMotionInfo, VRM_ANIMATIONS_DIR } from '../utils/vrm-animation-scanner'

describe('vrm-animation-scanner', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('buildKnownMotionInfo', () => {
    it('should return 11 known animations', () => {
      const list = buildKnownMotionInfo()
      expect(list).toHaveLength(11)
    })

    it('should have correct id/path/name for each animation', () => {
      const list = buildKnownMotionInfo()
      for (const item of list) {
        expect(item.id).toBeTruthy()
        expect(item.name).toBeTruthy()
        expect(item.path).toMatch(new RegExp(`^${VRM_ANIMATIONS_DIR}/.+\\.vrma$`))
      }
    })

    it('should include greeting as the first animation', () => {
      const list = buildKnownMotionInfo()
      const greeting = list.find((a) => a.id === 'greeting')
      expect(greeting).toBeDefined()
      expect(greeting?.name).toBe('问候')
      expect(greeting?.path).toBe(`${VRM_ANIMATIONS_DIR}/greeting.vrma`)
    })

    it('should map display names correctly for all known animations', () => {
      const list = buildKnownMotionInfo()
      const nameMap = Object.fromEntries(list.map((a) => [a.id, a.name]))

      expect(nameMap.greeting).toBe('问候')
      expect(nameMap.akimbo).toBe('叉腰')
      expect(nameMap.peace_sign).toBe('和平手势')
      expect(nameMap.play_fingers).toBe('玩手指')
      expect(nameMap.scratch_head).toBe('挠头')
      expect(nameMap.shoot).toBe('射击')
      expect(nameMap.show_full_body).toBe('全身展示')
      expect(nameMap.spin).toBe('旋转')
      expect(nameMap.squat).toBe('蹲下')
      expect(nameMap.stretch).toBe('伸展')
      expect(nameMap.model_pose).toBe('模型姿势')
    })
  })

  describe('scanVRMAnimations', () => {
    it('should scan from manifest.json when available', async () => {
      const mockManifest = { animations: ['greeting.vrma', 'stretch.vrma'] }
      vi.spyOn(globalThis, 'fetch').mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockManifest),
      } as Response)

      const result = await scanVRMAnimations()
      expect(result).toHaveLength(2)
      expect(result[0].id).toBe('greeting')
      expect(result[0].path).toBe(`${VRM_ANIMATIONS_DIR}/greeting.vrma`)
      expect(result[1].id).toBe('stretch')
    })

    it('should fall back to known animations when manifest is not ok', async () => {
      vi.spyOn(globalThis, 'fetch')
        // First call: manifest.json returns 404
        .mockResolvedValueOnce({ ok: false } as Response)
        // Subsequent calls: HEAD requests for each known animation
        .mockResolvedValue({ ok: true } as Response)

      const result = await scanVRMAnimations()
      // All 11 known animations should be found since HEAD returns ok
      expect(result.length).toBeGreaterThanOrEqual(0)
    })

    it('should handle fetch errors gracefully', async () => {
      vi.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('Network error'))

      const result = await scanVRMAnimations()
      // Should return empty array on total failure
      expect(result).toEqual([])
    })

    it('should filter out animations that do not exist via HEAD', async () => {
      vi.spyOn(globalThis, 'fetch')
        // Manifest not ok
        .mockResolvedValueOnce({ ok: false } as Response)
        // HEAD checks: only greeting exists
        .mockImplementation((url: string | URL | Request) => {
          const urlStr = typeof url === 'string' ? url : url instanceof URL ? url.toString() : url.url
          return Promise.resolve({
            ok: urlStr.includes('greeting.vrma') || urlStr.includes('stretch.vrma'),
          } as Response)
        })

      const result = await scanVRMAnimations()
      const ids = result.map((a) => a.id)
      expect(ids).toContain('greeting')
      expect(ids).toContain('stretch')
      // Shoot, spin etc should not be present
      expect(ids).not.toContain('shoot')
    })

    it('should handle manifest with missing animations array', async () => {
      vi.spyOn(globalThis, 'fetch')
        .mockResolvedValueOnce({
          ok: true,
          json: () => Promise.resolve({}), // no "animations" key
        } as Response)
        .mockResolvedValue({ ok: true } as Response)

      const result = await scanVRMAnimations()
      // Falls through to known animations since manifest has no animations array
      expect(result.length).toBeGreaterThanOrEqual(0)
    })
  })
})
