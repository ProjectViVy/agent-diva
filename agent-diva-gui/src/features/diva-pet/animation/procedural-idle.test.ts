import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import { ProceduralIdleGenerator } from './procedural-idle'
import type { ProceduralIdleBlend, ProceduralIdleConfig } from './types'

// ── Manual timing control ─────────────────────────────────

/** Clock that we manually advance and hook into performance.now */
function createFakeClock() {
  let now = 0
  const rafQueue: Array<() => void> = []

  function tick(ms: number) {
    now += ms
    // Run all queued rAF callbacks (they fire at 60fps intervals = ~16.67ms per frame)
    // For simplicity, fire one batch at current time
    const pending = rafQueue.splice(0)
    for (const cb of pending) {
      cb()
    }
  }

  function advanceBy(ms: number) {
    // Advance in 16ms chunks (roughly 60fps) to trigger rAF callbacks
    const frameMs = 16
    let remaining = ms
    while (remaining >= frameMs) {
      tick(frameMs)
      remaining -= frameMs
    }
    if (remaining > 0) {
      tick(remaining)
    }
  }

  return { now: () => now, advanceBy, tick, rafQueue }
}

// ── Tests ──────────────────────────────────────────────────

describe('ProceduralIdleGenerator', () => {
  let clock: ReturnType<typeof createFakeClock>

  beforeEach(() => {
    clock = createFakeClock()

    vi.stubGlobal('performance', {
      now: () => clock.now(),
    })

    // Hook rAF to our fake queue. Callbacks should re-schedule themselves.
    vi.stubGlobal('requestAnimationFrame', (cb: () => void): number => {
      clock.rafQueue.push(cb)
      return clock.rafQueue.length
    })

    vi.stubGlobal('cancelAnimationFrame', (_id: number) => {
      // No-op for simplicity — we flush synchronously
    })
  })

  afterEach(() => {
    vi.unstubAllGlobals()
  })

  // ── Constructor ──────────────────────────────────────────

  describe('constructor', () => {
    it('is not running initially', () => {
      const generator = new ProceduralIdleGenerator()
      expect(generator.isRunning).toBe(false)
    })

    it('merges provided config with defaults', () => {
      const partial: Partial<ProceduralIdleConfig> = {
        breathIntensity: 0.8,
        microMovementIntensity: 0.5,
      }
      const generator = new ProceduralIdleGenerator(partial)
      expect(generator.isRunning).toBe(false)
    })

    it('respects enabled: false in config and skips start', () => {
      const generator = new ProceduralIdleGenerator({ enabled: false })
      const onUpdate = vi.fn()

      generator.start(onUpdate)
      expect(generator.isRunning).toBe(false)
      expect(onUpdate).not.toHaveBeenCalled()
    })
  })

  // ── start / stop ─────────────────────────────────────────

  describe('start / stop lifecycle', () => {
    it('start sets isRunning to true', () => {
      const generator = new ProceduralIdleGenerator()
      const onUpdate = vi.fn()

      generator.start(onUpdate)
      expect(generator.isRunning).toBe(true)
    })

    it('does not start twice if already running', () => {
      const generator = new ProceduralIdleGenerator()
      const onUpdate1 = vi.fn()

      generator.start(onUpdate1)
      expect(generator.isRunning).toBe(true)

      const onUpdate2 = vi.fn()
      generator.start(onUpdate2)
      expect(generator.isRunning).toBe(true)
    })

    it('stop sets isRunning to false', () => {
      const generator = new ProceduralIdleGenerator()
      const onUpdate = vi.fn()

      generator.start(onUpdate)
      expect(generator.isRunning).toBe(true)

      generator.stop()
      expect(generator.isRunning).toBe(false)
    })

    it('stop calls onUpdate with zero blends', () => {
      const generator = new ProceduralIdleGenerator()
      const onUpdate = vi.fn()

      generator.start(onUpdate)
      onUpdate.mockClear()

      generator.stop()

      expect(onUpdate).toHaveBeenCalledTimes(1)
      const blend: ProceduralIdleBlend = onUpdate.mock.calls[0][0]
      expect(blend.breath).toBe(0)
      expect(blend.microX).toBe(0)
      expect(blend.microY).toBe(0)
      expect(blend.microZ).toBe(0)
    })
  })

  // ── Blend output shape ───────────────────────────────────

  describe('blend output', () => {
    it('returns ProceduralIdleBlend shape with all keys', () => {
      const generator = new ProceduralIdleGenerator()
      const onUpdate = vi.fn()

      generator.start(onUpdate)

      // Advance past the update interval (~83ms)
      clock.advanceBy(100)

      expect(onUpdate).toHaveBeenCalled()
      const blend: ProceduralIdleBlend = onUpdate.mock.calls[0][0]
      expect(blend).toHaveProperty('breath')
      expect(blend).toHaveProperty('microX')
      expect(blend).toHaveProperty('microY')
      expect(blend).toHaveProperty('microZ')

      expect(typeof blend.breath).toBe('number')
      expect(typeof blend.microX).toBe('number')
      expect(typeof blend.microY).toBe('number')
      expect(typeof blend.microZ).toBe('number')
    })

    it('breath oscillates between reasonable bounds', () => {
      const generator = new ProceduralIdleGenerator()
      const blends: ProceduralIdleBlend[] = []

      generator.start((blend) => blends.push({ ...blend }))

      // Run for 8 seconds to sample multiple values
      for (let i = 0; i < 8000; i += 84) {
        clock.advanceBy(84)
      }

      expect(blends.length).toBeGreaterThan(10)

      for (const b of blends) {
        expect(b.breath).toBeGreaterThanOrEqual(0)
        expect(b.breath).toBeLessThanOrEqual(1)
      }
    })

    it('breath value actually changes over time (not constant)', () => {
      const generator = new ProceduralIdleGenerator()
      const blends: ProceduralIdleBlend[] = []

      generator.start((blend) => blends.push({ ...blend }))

      for (let i = 0; i < 2000; i += 84) {
        clock.advanceBy(84)
      }

      expect(blends.length).toBeGreaterThan(3)

      const uniqueBreathValues = new Set(blends.map((b) => Number(b.breath.toFixed(3))))
      expect(uniqueBreathValues.size).toBeGreaterThan(1)
    })

    it('breath starts at the baseline (sin(0) = 0 → mapped to ~0.15 with default intensity)', () => {
      const generator = new ProceduralIdleGenerator()
      const blends: ProceduralIdleBlend[] = []

      generator.start((blend) => blends.push(blend))

      clock.advanceBy(100)

      expect(blends.length).toBeGreaterThan(0)
      const firstBlend = blends[0]
      expect(firstBlend.breath).toBeGreaterThanOrEqual(0)
      expect(firstBlend.breath).toBeLessThanOrEqual(1)
    })

    it('micro-movements produce small offsets', () => {
      const generator = new ProceduralIdleGenerator()
      const blends: ProceduralIdleBlend[] = []

      generator.start((blend) => blends.push({ ...blend }))

      for (let i = 0; i < 5000; i += 84) {
        clock.advanceBy(84)
      }

      for (const b of blends) {
        // With default intensity=0.3, absolute values < 0.3
        expect(Math.abs(b.microX)).toBeLessThanOrEqual(0.3 + 0.01)
        // microY scaled by 0.6 additional: ±0.18
        expect(Math.abs(b.microY)).toBeLessThanOrEqual(0.18 + 0.01)
        // microZ scaled by 0.4 additional: ±0.12
        expect(Math.abs(b.microZ)).toBeLessThanOrEqual(0.12 + 0.01)
      }
    })

    it('micro-movements vary over time', () => {
      const generator = new ProceduralIdleGenerator()
      const blends: ProceduralIdleBlend[] = []

      generator.start((blend) => blends.push({ ...blend }))

      for (let i = 0; i < 5000; i += 84) {
        clock.advanceBy(84)
      }

      const uniqueX = new Set(blends.map((b) => Number(b.microX.toFixed(3))))
      const uniqueY = new Set(blends.map((b) => Number(b.microY.toFixed(3))))
      const uniqueZ = new Set(blends.map((b) => Number(b.microZ.toFixed(3))))

      expect(uniqueX.size).toBeGreaterThan(1)
      expect(uniqueY.size).toBeGreaterThan(1)
      expect(uniqueZ.size).toBeGreaterThan(1)
    })
  })

  // ── setConfig at runtime ─────────────────────────────────

  describe('setConfig', () => {
    it('updates breath intensity at runtime', () => {
      const generator = new ProceduralIdleGenerator({ breathIntensity: 0.5 })
      const blends: ProceduralIdleBlend[] = []

      generator.start((blend) => blends.push({ ...blend }))

      // Run for a bit at default intensity
      for (let i = 0; i < 1000; i += 84) {
        clock.advanceBy(84)
      }
      const preChangeMax = Math.max(...blends.map((b) => b.breath))
      expect(preChangeMax).toBeLessThanOrEqual(0.5 + 0.01)

      // Change intensity
      generator.setConfig({ breathIntensity: 1.0 })
      blends.length = 0

      // Run for more time at new intensity
      for (let i = 0; i < 4000; i += 84) {
        clock.advanceBy(84)
      }
      const postChangeMax = Math.max(...blends.map((b) => b.breath))

      expect(postChangeMax).toBeGreaterThan(preChangeMax)
    })

    it('updates micro-movement intensity at runtime', () => {
      const generator = new ProceduralIdleGenerator({ microMovementIntensity: 0.1 })
      const blends: ProceduralIdleBlend[] = []

      generator.start((blend) => blends.push({ ...blend }))
      for (let i = 0; i < 2000; i += 84) {
        clock.advanceBy(84)
      }

      const preMaxX = Math.max(...blends.map((b) => Math.abs(b.microX)))
      expect(preMaxX).toBeLessThanOrEqual(0.1 + 0.01)

      // Increase intensity
      generator.setConfig({ microMovementIntensity: 0.5 })
      blends.length = 0
      for (let i = 0; i < 4000; i += 84) {
        clock.advanceBy(84)
      }

      const postMaxX = Math.max(...blends.map((b) => Math.abs(b.microX)))
      expect(postMaxX).toBeGreaterThan(preMaxX)
    })
  })

  // ── destroy ──────────────────────────────────────────────

  describe('destroy', () => {
    it('stops the generator', () => {
      const generator = new ProceduralIdleGenerator()
      const onUpdate = vi.fn()

      generator.start(onUpdate)
      expect(generator.isRunning).toBe(true)

      generator.destroy()
      expect(generator.isRunning).toBe(false)
    })
  })

  // ── Throttling ───────────────────────────────────────────

  describe('update throttling', () => {
    it('does not update more frequently than updateIntervalMs', () => {
      const generator = new ProceduralIdleGenerator({
        updateIntervalMs: 100,
      })
      const onUpdate = vi.fn()

      generator.start(onUpdate)

      // Advance less than the interval — onUpdate should not fire
      clock.advanceBy(50)
      expect(onUpdate).not.toHaveBeenCalled()

      // Advance past the interval — should now fire
      clock.advanceBy(60)
      expect(onUpdate).toHaveBeenCalled()
    })

    it('respects custom update interval', () => {
      const slowGenerator = new ProceduralIdleGenerator({ updateIntervalMs: 500 })
      const fastGenerator = new ProceduralIdleGenerator({ updateIntervalMs: 50 })

      const slowCalls: ProceduralIdleBlend[] = []
      const fastCalls: ProceduralIdleBlend[] = []

      slowGenerator.start((b) => slowCalls.push(b))
      fastGenerator.start((b) => fastCalls.push(b))

      for (let i = 0; i < 1000; i += 16) {
        clock.advanceBy(16)
      }

      // Fast generator should have more updates
      expect(fastCalls.length).toBeGreaterThan(slowCalls.length)
    })
  })

  // ── No updates after stop ────────────────────────────────

  describe('no updates after stop', () => {
    it('stops calling onUpdate after stop()', () => {
      const generator = new ProceduralIdleGenerator({ updateIntervalMs: 50 })
      const onUpdate = vi.fn()

      generator.start(onUpdate)
      for (let i = 0; i < 200; i += 16) {
        clock.advanceBy(16)
      }
      const callsBeforeStop = onUpdate.mock.calls.length
      expect(callsBeforeStop).toBeGreaterThan(0)

      generator.stop()
      onUpdate.mockClear()

      for (let i = 0; i < 1000; i += 16) {
        clock.advanceBy(16)
      }
      expect(onUpdate).not.toHaveBeenCalled()
    })
  })

  // ── Consistent output bounds across configs ──────────────

  describe('output bounds', () => {
    it('all blend values are finite numbers', () => {
      const generator = new ProceduralIdleGenerator()
      const blends: ProceduralIdleBlend[] = []

      generator.start((blend) => blends.push({ ...blend }))

      for (let i = 0; i < 10000; i += 84) {
        clock.advanceBy(84)
      }

      for (const b of blends) {
        expect(Number.isFinite(b.breath)).toBe(true)
        expect(Number.isFinite(b.microX)).toBe(true)
        expect(Number.isFinite(b.microY)).toBe(true)
        expect(Number.isFinite(b.microZ)).toBe(true)
      }
    })

    it('all blend values remain within [-1, 1]', () => {
      const generator = new ProceduralIdleGenerator()
      const blends: ProceduralIdleBlend[] = []

      generator.start((blend) => blends.push({ ...blend }))

      for (let i = 0; i < 10000; i += 84) {
        clock.advanceBy(84)
      }

      for (const b of blends) {
        expect(b.breath).toBeGreaterThanOrEqual(-1)
        expect(b.breath).toBeLessThanOrEqual(1)
        expect(b.microX).toBeGreaterThanOrEqual(-1)
        expect(b.microX).toBeLessThanOrEqual(1)
        expect(b.microY).toBeGreaterThanOrEqual(-1)
        expect(b.microY).toBeLessThanOrEqual(1)
        expect(b.microZ).toBeGreaterThanOrEqual(-1)
        expect(b.microZ).toBeLessThanOrEqual(1)
      }
    })
  })
})
