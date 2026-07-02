/**
 * Lip Sync Analyzer — Phase 3 core: F1/F2 formant-based vowel detection.
 *
 * Converts live audio (via Web Audio API AnalyserNode) into viseme
 * blend weights using a vowel-triangle mapping, then passes the
 * dominant viseme back through a callback so the VRM runtime can
 * drive mouth BlendShapes.
 *
 * Reference: super-agent-party vrm.js:1537-1704 (formant lip sync)
 *
 * Integration: Works with avatar-runtime-vrm's AvatarSpeechState.viseme
 *              which maps directly to BlendShape names (aa/ih/ou/ee/oh).
 */

// ── Public types ──────────────────────────────────────────────────

/** Vowel blend weights produced by the analyzer each frame */
export interface VowelBlends {
  /** "啊" — wide open mouth (F1 > 600 Hz) */
  aa: number
  /** "一" — tongue forward, mouth narrow (F1 < 450, F2 > 1800) */
  ih: number
  /** "呜" — tongue back, lips rounded (F1 < 450, F2 < 1100) */
  ou: number
  /** "耶" — high F2 (F2 > 1600) */
  ee: number
  /** "哦" — neutral / mid position */
  oh: number
}

/** Configuration for the lip sync analyzer */
export interface LipSyncConfig {
  /** Overall sensitivity multiplier (0.5–2.0) */
  sensitivity: number
  /** Noise gate threshold: blends below this FFT energy are silenced */
  noiseGate: number
  /** Smoothing factor when opening mouth (higher = faster, 0–1) */
  smoothOpen: number
  /** Smoothing factor when closing mouth (lower = faster snap-shut, 0–1) */
  smoothClose: number
  /** FFT size for frequency analysis (power of 2) */
  fftSize: number
  /** Whether the analyzer is enabled */
  enabled: boolean
}

// ── Defaults (matching reference implementation) ──────────────────

export const DEFAULT_LIP_SYNC_CONFIG: LipSyncConfig = {
  sensitivity: 1.0,
  noiseGate: 15,
  smoothOpen: 0.5,
  smoothClose: 0.1,
  fftSize: 1024,
  enabled: true,
}

// ── Internal helpers ──────────────────────────────────────────────

interface Formant {
  /** Detected frequency in Hz */
  freq: number
  /** Amplitude of the peak (0–255) */
  amp: number
}

// ── LipSyncAnalyzer ───────────────────────────────────────────────

export class LipSyncAnalyzer {
  // ── Audio graph ────────────────────────────────────────────────

  private audioContext: AudioContext | null = null
  private analyser: AnalyserNode | null = null
  // Cached reference to the connected source node (for lifecycle)
  private sourceNode: MediaElementAudioSourceNode | null = null

  // ── State ──────────────────────────────────────────────────────

  private animationId: number | null = null
  private currentBlends: VowelBlends = { aa: 0, ih: 0, ou: 0, ee: 0, oh: 0 }
  private buffer: Uint8Array | null = null
  private config: LipSyncConfig
  private onUpdate: ((blends: VowelBlends) => void) | null = null
  private isAnalyzing = false

  // ── Constructor ────────────────────────────────────────────────

  constructor(config?: Partial<LipSyncConfig>) {
    this.config = { ...DEFAULT_LIP_SYNC_CONFIG, ...config }
  }

  // ── Configuration ──────────────────────────────────────────────

  /** Update config at runtime (partial merge) */
  setConfig(patch: Partial<LipSyncConfig>): void {
    this.config = { ...this.config, ...patch }
    if (this.analyser) {
      this.analyser.fftSize = this.config.fftSize
      this.buffer = new Uint8Array(this.analyser.frequencyBinCount)
    }
  }

  /** Get current config snapshot */
  getConfig(): Readonly<LipSyncConfig> {
    return this.config
  }

  // ── Audio source connection ────────────────────────────────────

  /**
   * Connect an HTMLAudioElement as the analysis source.
   *
   * Creates a MediaElementAudioSourceNode from the element and pipes
   * it through an AnalyserNode to the AudioContext destination.
   *
   * Must be called BEFORE the audio element starts playing.
   * Only one source may be connected at a time; calling this again
   * disconnects the previous source.
   */
  connectMediaElement(audioElement: HTMLAudioElement): void {
    if (!this.config.enabled) return

    // Lazily create AudioContext (avoids auto-play policy issues)
    if (!this.audioContext) {
      this.audioContext = new AudioContext()
    }

    // Disconnect previous source if any
    this.disconnectSource()

    try {
      // Connect audio element → analyser → destination
      const ctx = this.audioContext
      this.analyser = ctx.createAnalyser()
      this.analyser.fftSize = this.config.fftSize
      this.analyser.smoothingTimeConstant = 0.3
      this.buffer = new Uint8Array(this.analyser.frequencyBinCount)

      const source = ctx.createMediaElementSource(audioElement)
      source.connect(this.analyser)
      this.analyser.connect(ctx.destination)
      this.sourceNode = source

      console.log('[LipSyncAnalyzer] Connected to audio element')
    } catch (error) {
      console.warn('[LipSyncAnalyzer] Failed to connect media element source', error)
      this.analyser = null
    }
  }

  /**
   * Connect a raw AudioNode as the analysis source.
   *
   * For use when the audio is already being processed through
   * a Web Audio graph (e.g., microphone input via MediaStreamSource).
   * The node will be connected: sourceNode → analyser → destination.
   */
  connectNode(sourceNode: AudioNode): void {
    if (!this.config.enabled) return

    if (!this.audioContext) {
      this.audioContext = new AudioContext()
    }

    this.disconnectSource()

    const ctx = this.audioContext
    this.analyser = ctx.createAnalyser()
    this.analyser.fftSize = this.config.fftSize
    this.analyser.smoothingTimeConstant = 0.3
    this.buffer = new Uint8Array(this.analyser.frequencyBinCount)

    sourceNode.connect(this.analyser)
    this.analyser.connect(ctx.destination)

    console.log('[LipSyncAnalyzer] Connected to AudioNode')
  }

  /** Disconnect the current source */
  private disconnectSource(): void {
    if (this.sourceNode) {
      try { this.sourceNode.disconnect() } catch { /* ignore */ }
      this.sourceNode = null
    }
    if (this.analyser) {
      try { this.analyser.disconnect() } catch { /* ignore */ }
      this.analyser = null
    }
  }

  // ── Analysis lifecycle ─────────────────────────────────────────

  /**
   * Start the real-time analysis loop.
   *
   * Must be called after connecting an audio source.
   * The callback receives per-frame vowel blend weights.
   */
  start(onUpdate: (blends: VowelBlends) => void): void {
    if (!this.analyser || !this.buffer) {
      console.warn('[LipSyncAnalyzer] Cannot start: no audio source connected')
      return
    }

    if (this.isAnalyzing) {
      console.log('[LipSyncAnalyzer] Already analyzing, restarting')
      this.stop()
    }

    this.onUpdate = onUpdate
    this.isAnalyzing = true
    this.animationId = requestAnimationFrame(() => this.analyzeFrame())
    console.log('[LipSyncAnalyzer] Analysis started')
  }

  /** Stop the analysis loop and reset blends to zero */
  stop(): void {
    this.isAnalyzing = false
    if (this.animationId !== null) {
      cancelAnimationFrame(this.animationId)
      this.animationId = null
    }
    // Reset blends to silence
    this.currentBlends = { aa: 0, ih: 0, ou: 0, ee: 0, oh: 0 }
    this.onUpdate?.(this.currentBlends)
    console.log('[LipSyncAnalyzer] Analysis stopped')
  }

  /** Whether the analysis loop is currently running */
  get active(): boolean {
    return this.isAnalyzing
  }

  // ── Per-frame analysis ─────────────────────────────────────────

  private analyzeFrame(): void {
    if (!this.isAnalyzing || !this.analyser || !this.buffer) return

    // Schedule next frame
    this.animationId = requestAnimationFrame(() => this.analyzeFrame())

    // Read current frequency data
    this.analyser.getByteFrequencyData(this.buffer)
    const sampleRate = this.audioContext?.sampleRate ?? 44100

    // Compute F1 (200–1000 Hz) and F2 (1000–3000 Hz) formants
    const f1 = this.getFormant(200, 1000, sampleRate)
    const f2 = this.getFormant(1000, 3000, sampleRate)

    // Compute vocal energy in the speech band (200–4000 Hz)
    const vocalEnergy = this.getVocalEnergy(sampleRate)

    // ── Vowel triangle mapping ──────────────────────────────────
    // Reference: vrm.js:1462–1491

    let target: VowelBlends = { aa: 0, ih: 0, ou: 0, ee: 0, oh: 0 }

    if (vocalEnergy > this.config.noiseGate) {
      const intensity = Math.min(1.0, (vocalEnergy / 255) * this.config.sensitivity)

      if (f1.freq > 600) {
        // Open / low vowel: "aa" (啊)
        target.aa = intensity
      } else if (f1.freq < 450 && f2.freq > 1800) {
        // High front vowel: "ih" (一)
        target.ih = intensity
        target.ee = intensity * 0.3
      } else if (f1.freq < 450 && f2.freq < 1100) {
        // High back rounded vowel: "ou" (呜)
        target.ou = intensity
      } else if (f2.freq > 1600) {
        // Front vowel: "ee" (耶)
        target.ee = intensity
        target.ih = intensity * 0.2
      } else {
        // Mid / neutral: "oh" (哦)
        target.oh = intensity
        target.ou = intensity * 0.3
      }
    }

    // ── Smooth interpolation ────────────────────────────────────
    // Asymmetric smoothing: faster open, slower close (natural)
    this.currentBlends = {
      aa: this.smooth(this.currentBlends.aa, target.aa),
      ih: this.smooth(this.currentBlends.ih, target.ih),
      ou: this.smooth(this.currentBlends.ou, target.ou),
      ee: this.smooth(this.currentBlends.ee, target.ee),
      oh: this.smooth(this.currentBlends.oh, target.oh),
    }

    this.onUpdate?.(this.currentBlends)
  }

  // ── Formant detection ──────────────────────────────────────────

  /**
   * Find the dominant frequency peak within a given band.
   */
  private getFormant(minFreq: number, maxFreq: number, sampleRate: number): Formant {
    if (!this.buffer) return { freq: 0, amp: 0 }

    const nyquist = sampleRate / 2
    const bufferLength = this.buffer.length
    const startIdx = Math.max(0, Math.floor((minFreq / nyquist) * bufferLength))
    const endIdx = Math.min(bufferLength - 1, Math.floor((maxFreq / nyquist) * bufferLength))

    let maxAmp = -Infinity
    let maxIdx = -1

    for (let i = startIdx; i <= endIdx; i++) {
      if (this.buffer[i] > maxAmp) {
        maxAmp = this.buffer[i]
        maxIdx = i
      }
    }

    return {
      freq: maxIdx >= 0 ? (maxIdx / bufferLength) * nyquist : 0,
      amp: maxIdx >= 0 ? maxAmp : 0,
    }
  }

  /**
   * Compute average energy in the speech-relevant band (200–4000 Hz).
   */
  private getVocalEnergy(sampleRate: number): number {
    if (!this.buffer) return 0

    const nyquist = sampleRate / 2
    const bufferLength = this.buffer.length
    const startBin = Math.max(0, Math.floor((200 / nyquist) * bufferLength))
    const endBin = Math.min(bufferLength - 1, Math.floor((4000 / nyquist) * bufferLength))

    let energy = 0
    for (let i = startBin; i <= endBin; i++) {
      energy += this.buffer[i]
    }
    return energy / Math.max(1, endBin - startBin)
  }

  // ── Smoothing ──────────────────────────────────────────────────

  /**
   * Asymmetric lerp: faster open (smoothOpen), slower close (smoothClose).
   */
  private smooth(current: number, target: number): number {
    const factor = target > current ? this.config.smoothOpen : this.config.smoothClose
    return current + (target - current) * factor
  }

  // ── Utility: best viseme ───────────────────────────────────────

  /**
   * Pick the dominant viseme from the current blends.
   *
   * Returns null if all blends are below a minimum threshold (silence).
   * Matches avatar-runtime-vrm's AvatarViseme union: 'aa'|'ih'|'ou'|'ee'|'oh'
   */
  static getDominantViseme(blends: VowelBlends, threshold = 0.05): string | null {
    let best: keyof VowelBlends | null = null
    let max = 0

    for (const key of Object.keys(blends) as Array<keyof VowelBlends>) {
      if (blends[key] > max) {
        max = blends[key]
        best = key
      }
    }

    return best && max >= threshold ? best : null
  }

  // ── Destroy ────────────────────────────────────────────────────

  /** Fully destroy the analyzer and release all resources */
  destroy(): void {
    this.stop()
    this.disconnectSource()

    if (this.audioContext) {
      // AudioContext.close() is async but we fire-and-forget on destroy
      void this.audioContext.close().catch(() => { /* ignore close errors */ })
      this.audioContext = null
    }

    this.buffer = null
    this.onUpdate = null
    this.sourceNode = null
    console.log('[LipSyncAnalyzer] Destroyed')
  }
}
