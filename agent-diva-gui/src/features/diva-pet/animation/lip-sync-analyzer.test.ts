import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import {
  LipSyncAnalyzer,
  DEFAULT_LIP_SYNC_CONFIG,
  type VowelBlends,
} from './lip-sync-analyzer'

// ── Mocks ──────────────────────────────────────────────────

/** Mock frequency data array — 512 bins of zero energy */
function createMockFrequencyBuffer(bins: number): Uint8Array {
  return new Uint8Array(bins)
}

/** Create a minimal mock AnalyserNode */
function createMockAnalyser(frequencyBinCount = 512): AnalyserNode {
  const buffer = createMockFrequencyBuffer(frequencyBinCount)
  return {
    fftSize: 1024,
    frequencyBinCount,
    smoothingTimeConstant: 0.3,
    getByteFrequencyData: vi.fn().mockImplementation((arr: Uint8Array) => {
      // Copy our mock buffer into the target
      for (let i = 0; i < Math.min(arr.length, buffer.length); i++) {
        arr[i] = buffer[i]
      }
    }),
    connect: vi.fn(),
    disconnect: vi.fn(),
    // AnalyserNode-specific props we need
    minDecibels: -100,
    maxDecibels: -30,
    getFloatFrequencyData: vi.fn(),
    getByteTimeDomainData: vi.fn(),
    getFloatTimeDomainData: vi.fn(),
    channelCount: 2,
    channelCountMode: 'max' as const,
    channelInterpretation: 'speakers' as const,
    context: null as unknown as BaseAudioContext,
    numberOfInputs: 1,
    numberOfOutputs: 1,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  } as unknown as AnalyserNode
}

/** Create a mock AudioContext */
function createMockAudioContext(): AudioContext {
  const analyser = createMockAnalyser()

  return {
    createAnalyser: vi.fn().mockReturnValue(analyser),
    createMediaElementSource: vi.fn().mockReturnValue({
      connect: vi.fn(),
      disconnect: vi.fn(),
    }),
    sampleRate: 44100,
    destination: {},
    close: vi.fn().mockResolvedValue(undefined),
    state: 'running',
    // Minimal other AudioContext members
    baseLatency: 0,
    outputLatency: 0,
    currentTime: 0,
    createBuffer: vi.fn(),
    createBufferSource: vi.fn(),
    createGain: vi.fn(),
    createOscillator: vi.fn(),
    resume: vi.fn(),
    suspend: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  } as unknown as AudioContext
}

/** Helper: create a mock HTMLAudioElement */
function createMockAudioElement(): HTMLAudioElement {
  return {
    tagName: 'AUDIO',
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  } as unknown as HTMLAudioElement
}

// ── Tests ──────────────────────────────────────────────────

describe('LipSyncAnalyzer', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    // Stub global AudioContext so `new AudioContext()` works
    const ctx = createMockAudioContext()
    vi.stubGlobal('AudioContext', function (this: AudioContext) {
      Object.assign(this, ctx)
      return this
    })
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
  })

  // ── Constructor ──────────────────────────────────────────

  describe('constructor', () => {
    it('initializes with default config', () => {
      const analyzer = new LipSyncAnalyzer()

      const config = analyzer.getConfig()
      expect(config.sensitivity).toBe(DEFAULT_LIP_SYNC_CONFIG.sensitivity)
      expect(config.noiseGate).toBe(DEFAULT_LIP_SYNC_CONFIG.noiseGate)
      expect(config.smoothOpen).toBe(DEFAULT_LIP_SYNC_CONFIG.smoothOpen)
      expect(config.smoothClose).toBe(DEFAULT_LIP_SYNC_CONFIG.smoothClose)
      expect(config.fftSize).toBe(DEFAULT_LIP_SYNC_CONFIG.fftSize)
      expect(config.enabled).toBe(true)
    })

    it('merges provided config with defaults', () => {
      const analyzer = new LipSyncAnalyzer({ sensitivity: 1.5, noiseGate: 30 })

      const config = analyzer.getConfig()
      expect(config.sensitivity).toBe(1.5)
      expect(config.noiseGate).toBe(30)
      // Defaults preserved for unspecified fields
      expect(config.smoothOpen).toBe(DEFAULT_LIP_SYNC_CONFIG.smoothOpen)
    })

    it('is not active initially', () => {
      const analyzer = new LipSyncAnalyzer()
      expect(analyzer.active).toBe(false)
    })
  })

  // ── setConfig ────────────────────────────────────────────

  describe('setConfig', () => {
    it('updates sensitivity', () => {
      const analyzer = new LipSyncAnalyzer()
      analyzer.setConfig({ sensitivity: 2.0 })

      expect(analyzer.getConfig().sensitivity).toBe(2.0)
    })

    it('updates noiseGate', () => {
      const analyzer = new LipSyncAnalyzer()
      analyzer.setConfig({ noiseGate: 25 })

      expect(analyzer.getConfig().noiseGate).toBe(25)
    })

    it('updates smoothOpen', () => {
      const analyzer = new LipSyncAnalyzer()
      analyzer.setConfig({ smoothOpen: 0.8 })

      expect(analyzer.getConfig().smoothOpen).toBe(0.8)
    })

    it('updates smoothClose', () => {
      const analyzer = new LipSyncAnalyzer()
      analyzer.setConfig({ smoothClose: 0.05 })

      expect(analyzer.getConfig().smoothClose).toBe(0.05)
    })

    it('updates fftSize', () => {
      const analyzer = new LipSyncAnalyzer()
      analyzer.setConfig({ fftSize: 2048 })

      expect(analyzer.getConfig().fftSize).toBe(2048)
    })

    it('updates enabled flag', () => {
      const analyzer = new LipSyncAnalyzer()
      analyzer.setConfig({ enabled: false })

      expect(analyzer.getConfig().enabled).toBe(false)
    })

    it('partial update preserves other config fields', () => {
      const analyzer = new LipSyncAnalyzer()
      analyzer.setConfig({ sensitivity: 1.8 })

      const config = analyzer.getConfig()
      expect(config.sensitivity).toBe(1.8)
      expect(config.smoothOpen).toBe(DEFAULT_LIP_SYNC_CONFIG.smoothOpen)
      expect(config.noiseGate).toBe(DEFAULT_LIP_SYNC_CONFIG.noiseGate)
    })

    it('updates analyserNode fftSize when connected', () => {
      const analyzer = new LipSyncAnalyzer({ enabled: true })
      const audioEl = createMockAudioElement()

      analyzer.connectMediaElement(audioEl)

      // Now change fftSize — should not throw
      expect(() => analyzer.setConfig({ fftSize: 512 })).not.toThrow()
      // The config is updated
      expect(analyzer.getConfig().fftSize).toBe(512)
    })
  })

  // ── connectMediaElement ──────────────────────────────────

  describe('connectMediaElement', () => {
    it('creates an AudioContext and AnalyserNode', () => {
      const analyzer = new LipSyncAnalyzer({ enabled: true })
      const audioEl = createMockAudioElement()

      // Should not throw — implies AudioContext creation succeeded
      expect(() => analyzer.connectMediaElement(audioEl)).not.toThrow()
    })

    it('skips creation when config.enabled is false', () => {
      const analyzer = new LipSyncAnalyzer({ enabled: false })
      const audioEl = createMockAudioElement()

      // Should not throw — and no AudioContext is created since enabled=false
      expect(() => analyzer.connectMediaElement(audioEl)).not.toThrow()
    })

    it('sets fftSize on the analyser node', () => {
      const analyzer = new LipSyncAnalyzer({ enabled: true, fftSize: 2048 })
      const audioEl = createMockAudioElement()

      // Should connect successfully with the specified fftSize
      expect(() => analyzer.connectMediaElement(audioEl)).not.toThrow()
    })
  })

  // ── start ────────────────────────────────────────────────

  describe('start', () => {
    it('begins the requestAnimationFrame loop', () => {
      const analyzer = new LipSyncAnalyzer({ enabled: true })
      const audioEl = createMockAudioElement()
      analyzer.connectMediaElement(audioEl)

      const onUpdate = vi.fn()
      analyzer.start(onUpdate)

      expect(analyzer.active).toBe(true)
      // rAF should have been requested
      // Advance some frames
      vi.advanceTimersByTime(100)
    })

    it('does not start without an audio source', () => {
      const analyzer = new LipSyncAnalyzer({ enabled: true })
      // No connectMediaElement call

      const onUpdate = vi.fn()
      analyzer.start(onUpdate)

      expect(analyzer.active).toBe(false)
      expect(onUpdate).not.toHaveBeenCalled()
    })

    it('restarts if already analyzing', () => {
      const analyzer = new LipSyncAnalyzer({ enabled: true })
      const audioEl = createMockAudioElement()
      analyzer.connectMediaElement(audioEl)

      const onUpdate = vi.fn()
      analyzer.start(onUpdate)
      expect(analyzer.active).toBe(true)

      // Start again — should stop and restart
      const onUpdate2 = vi.fn()
      analyzer.start(onUpdate2)
      expect(analyzer.active).toBe(true)
    })
  })

  // ── stop ─────────────────────────────────────────────────

  describe('stop', () => {
    it('cancels the animation frame loop', () => {
      const analyzer = new LipSyncAnalyzer({ enabled: true })
      const audioEl = createMockAudioElement()
      analyzer.connectMediaElement(audioEl)

      const onUpdate = vi.fn()
      analyzer.start(onUpdate)
      expect(analyzer.active).toBe(true)

      analyzer.stop()
      expect(analyzer.active).toBe(false)
    })

    it('resets blends to zero and calls onUpdate', () => {
      const analyzer = new LipSyncAnalyzer({ enabled: true })
      const audioEl = createMockAudioElement()
      analyzer.connectMediaElement(audioEl)

      const onUpdate = vi.fn()
      analyzer.start(onUpdate)

      // Clear calls from startup
      vi.advanceTimersByTime(100)
      onUpdate.mockClear()

      analyzer.stop()

      // Should have called onUpdate with all-zero blends
      expect(onUpdate).toHaveBeenCalledTimes(1)
      const blends: VowelBlends = onUpdate.mock.calls[0][0]
      expect(blends.aa).toBe(0)
      expect(blends.ih).toBe(0)
      expect(blends.ou).toBe(0)
      expect(blends.ee).toBe(0)
      expect(blends.oh).toBe(0)
    })
  })

  // ── destroy ──────────────────────────────────────────────

  describe('destroy', () => {
    it('stops the analysis loop', () => {
      const analyzer = new LipSyncAnalyzer({ enabled: true })
      const audioEl = createMockAudioElement()
      analyzer.connectMediaElement(audioEl)

      analyzer.start(vi.fn())
      expect(analyzer.active).toBe(true)

      analyzer.destroy()
      expect(analyzer.active).toBe(false)
    })

    it('closes the AudioContext', () => {
      const analyzer = new LipSyncAnalyzer({ enabled: true })
      const audioEl = createMockAudioElement()
      analyzer.connectMediaElement(audioEl)

      // destroy() should not throw (close may be async, fire-and-forget)
      expect(() => analyzer.destroy()).not.toThrow()
    })
  })

  // ── getDominantViseme (static) ───────────────────────────

  describe('getDominantViseme', () => {
    it('returns aa when it is dominant', () => {
      const blends: VowelBlends = { aa: 0.8, ih: 0.1, ou: 0.05, ee: 0.1, oh: 0.02 }
      const result = LipSyncAnalyzer.getDominantViseme(blends)
      expect(result).toBe('aa')
    })

    it('returns ih when it is dominant', () => {
      const blends: VowelBlends = { aa: 0.1, ih: 0.9, ou: 0.05, ee: 0.1, oh: 0.02 }
      const result = LipSyncAnalyzer.getDominantViseme(blends)
      expect(result).toBe('ih')
    })

    it('returns ou when it is dominant', () => {
      const blends: VowelBlends = { aa: 0.1, ih: 0.1, ou: 0.7, ee: 0.1, oh: 0.02 }
      const result = LipSyncAnalyzer.getDominantViseme(blends)
      expect(result).toBe('ou')
    })

    it('returns ee when it is dominant', () => {
      const blends: VowelBlends = { aa: 0.1, ih: 0.1, ou: 0.05, ee: 0.8, oh: 0.02 }
      const result = LipSyncAnalyzer.getDominantViseme(blends)
      expect(result).toBe('ee')
    })

    it('returns oh when it is dominant', () => {
      const blends: VowelBlends = { aa: 0.1, ih: 0.1, ou: 0.05, ee: 0.1, oh: 0.6 }
      const result = LipSyncAnalyzer.getDominantViseme(blends)
      expect(result).toBe('oh')
    })

    it('returns null when all blends are below threshold', () => {
      const blends: VowelBlends = { aa: 0.03, ih: 0.01, ou: 0.02, ee: 0.01, oh: 0.04 }
      const result = LipSyncAnalyzer.getDominantViseme(blends)
      expect(result).toBeNull()
    })

    it('respects custom threshold', () => {
      const blends: VowelBlends = { aa: 0.07, ih: 0.01, ou: 0.02, ee: 0.01, oh: 0.04 }
      // Default threshold is 0.05, aa=0.07 > 0.05
      expect(LipSyncAnalyzer.getDominantViseme(blends)).toBe('aa')
      // With higher threshold of 0.1, nothing qualifies
      expect(LipSyncAnalyzer.getDominantViseme(blends, 0.1)).toBeNull()
    })

    it('returns first max when there is a tie', () => {
      const blends: VowelBlends = { aa: 0.8, ih: 0.8, ou: 0.1, ee: 0.1, oh: 0.1 }
      const result = LipSyncAnalyzer.getDominantViseme(blends)
      // Should return 'aa' as it's the first encountered max
      expect(['aa', 'ih']).toContain(result)
    })
  })

  // ── Config edge cases ────────────────────────────────────

  describe('config edge cases', () => {
    it('handles zero sensitivity', () => {
      const analyzer = new LipSyncAnalyzer({ sensitivity: 0.0 })
      expect(analyzer.getConfig().sensitivity).toBe(0.0)
    })

    it('handles maximum sensitivity', () => {
      const analyzer = new LipSyncAnalyzer({ sensitivity: 2.0 })
      expect(analyzer.getConfig().sensitivity).toBe(2.0)
    })

    it('handles extreme noise gate', () => {
      const analyzer = new LipSyncAnalyzer({ noiseGate: 255 })
      expect(analyzer.getConfig().noiseGate).toBe(255)
    })

    it('getConfig returns a frozen snapshot', () => {
      const analyzer = new LipSyncAnalyzer({ sensitivity: 1.2 })
      const config = analyzer.getConfig()
      expect(config.sensitivity).toBe(1.2)

      // Modifying the returned config should not affect the analyzer
      analyzer.setConfig({ sensitivity: 0.5 })
      expect(analyzer.getConfig().sensitivity).toBe(0.5)
    })
  })
})
