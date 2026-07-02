import type { VRM } from '@pixiv/three-vrm'

interface ActiveChunkState {
  isPlaying: boolean
  animationId: number
  analyser: AnalyserNode
  audioContext: AudioContext
  expression: string | null
  currentBlends: { aa: number; ih: number; ou: number; ee: number; oh: number }
}

/**
 * Callbacks for chunk lifecycle events. Follows the same pattern as
 * {@link MotionControllerHooks} and {@link AnimationControllerHooks}.
 */
export interface ChunkControllerHooks {
  onChunkStart?: (chunkId: string, expression?: string) => void | Promise<void>
  onChunkEnd?: (chunkId: string, expression?: string) => void | Promise<void>
  onAllChunksEnd?: () => void | Promise<void>
}

const VOWEL_SHAPES = ['aa', 'ih', 'ou', 'ee', 'oh'] as const

const NOISE_GATE = 15

const EMOTIONS = ['surprised', 'happy', 'angry', 'sad', 'neutral', 'relaxed'] as const

export class ChunkAnimationController {
  private vrm: VRM | null = null
  private readonly chunks = new Map<string, ActiveChunkState>()
  private readonly hooks: ChunkControllerHooks

  constructor(hooks: ChunkControllerHooks = {}) {
    this.hooks = hooks
  }

  attach(vrm: VRM): void {
    this.vrm = vrm
    this.chunks.clear()
  }

  detach(): void {
    this.stopAll()
    this.chunks.clear()
    this.vrm = null
  }

  async playChunk(
    chunkId: string,
    audioBuffer: AudioBuffer,
    expression?: string,
  ): Promise<void> {
    // Stop existing chunk with the same ID before starting a new one
    this.stopChunk(chunkId)

    const audioContext = new AudioContext()

    if (audioContext.state === 'suspended') {
      await audioContext.resume()
    }

    const analyser = audioContext.createAnalyser()
    analyser.fftSize = 1024
    analyser.smoothingTimeConstant = 0.3

    const source = audioContext.createBufferSource()
    source.buffer = audioBuffer
    source.connect(analyser)
    analyser.connect(audioContext.destination)

    const chunkState: ActiveChunkState = {
      isPlaying: true,
      animationId: 0,
      analyser,
      audioContext,
      expression: expression ?? null,
      currentBlends: { aa: 0, ih: 0, ou: 0, ee: 0, oh: 0 },
    }

    this.chunks.set(chunkId, chunkState)

    source.onended = () => {
      this.stopChunk(chunkId)
    }

    this.startLoop(chunkId, chunkState, audioContext.sampleRate)

    source.start(0)

    this.hooks.onChunkStart?.(chunkId, expression)
  }

  stopChunk(chunkId: string): void {
    const chunkState = this.chunks.get(chunkId)
    if (!chunkState || !chunkState.isPlaying) {
      return
    }

    chunkState.isPlaying = false

    if (chunkState.animationId) {
      cancelAnimationFrame(chunkState.animationId)
      chunkState.animationId = 0
    }

    chunkState.analyser.disconnect()
    chunkState.audioContext.close()

    const endedExpression = chunkState.expression ?? undefined

    this.chunks.delete(chunkId)

    if (this.chunks.size === 0) {
      this.resetVowelBlendshapes()
    }

    this.hooks.onChunkEnd?.(chunkId, endedExpression)

    if (this.chunks.size === 0) {
      this.hooks.onAllChunksEnd?.()
    }
  }

  stopAll(): void {
    for (const chunkId of this.chunks.keys()) {
      this.stopChunk(chunkId)
    }
  }

  update(_deltaSeconds: number): void {
    // NO-OP: lip sync runs its own rAF loop per chunk
  }

  getActiveChunks(): number {
    return this.chunks.size
  }

  private startLoop(
    chunkId: string,
    state: ActiveChunkState,
    sampleRate: number,
  ): void {
    const analyser = state.analyser
    const bufferLength = analyser.frequencyBinCount
    const dataArray = new Uint8Array(bufferLength)

    const getFormant = (minFreq: number, maxFreq: number) => {
      const nyquist = sampleRate / 2
      const startIndex = Math.floor((minFreq / nyquist) * bufferLength)
      const endIndex = Math.floor((maxFreq / nyquist) * bufferLength)

      let maxAmp = -Infinity
      let maxIndex = -1

      for (let i = startIndex; i <= endIndex; i++) {
        if (dataArray[i] > maxAmp) {
          maxAmp = dataArray[i]
          maxIndex = i
        }
      }

      return {
        freq: (maxIndex / bufferLength) * nyquist,
        amp: maxAmp,
      }
    }

    const animate = () => {
      const currentState = this.chunks.get(chunkId)
      if (!currentState || !currentState.isPlaying) {
        this.resetVowelBlendshapes()
        return
      }

      currentState.animationId = requestAnimationFrame(animate)

      // 1. Get frequency domain data
      analyser.getByteFrequencyData(dataArray)

      // 2. Detect formants
      const f1 = getFormant(200, 1000)
      const f2 = getFormant(1000, 3000)

      // 3. Calculate vocal energy (200-4000Hz range)
      let vocalEnergy = 0
      const startBin = Math.floor((200 / (sampleRate / 2)) * bufferLength)
      const endBin = Math.floor((4000 / (sampleRate / 2)) * bufferLength)
      for (let i = startBin; i < endBin; i++) {
        vocalEnergy += dataArray[i]
      }
      const avgVol = vocalEnergy / (endBin - startBin)

      // 4. Vowel triangle mapping → target blendshapes
      const target = { aa: 0, ih: 0, ou: 0, ee: 0, oh: 0 }

      if (avgVol > NOISE_GATE) {
        const intensity = Math.min(1.0, (avgVol / 255) * 1.0)

        if (f1.freq > 600) {
          // F1 high → open mouth → "aa"
          target.aa = intensity
        } else if (f1.freq < 450 && f2.freq > 1800) {
          // F1 low (closed), F2 high (tongue front) → "ih" + slight "ee"
          target.ih = intensity
          target.ee = intensity * 0.3
        } else if (f1.freq < 450 && f2.freq < 1100) {
          // F1 low (closed), F2 low (tongue back) → "ou" (rounded back)
          target.ou = intensity
        } else if (f2.freq > 1600) {
          // Remaining high F2 → "ee" + slight "ih"
          target.ee = intensity
          target.ih = intensity * 0.2
        } else {
          // Default mid → "oh" + slight "ou"
          target.oh = intensity
          target.ou = intensity * 0.3
        }
      }

      // 5. Apply to VRM with smoothing + expression integration
      if (this.vrm?.expressionManager) {
        const expression = currentState.expression
        const limit =
          expression && (expression === 'happy' || expression === 'surprised')
            ? 0.5
            : 1.0

        // Apply emotion expression
        if (
          expression &&
          (EMOTIONS as readonly string[]).includes(expression)
        ) {
          for (const exp of EMOTIONS) {
            this.vrm.expressionManager.setValue(
              exp,
              exp === expression ? 1.0 : 0.0,
            )
          }
        }

        // Apply vowel blendshapes with smooth interpolation
        for (const v of VOWEL_SHAPES) {
          const t = target[v] * limit
          const c = currentState.currentBlends[v]
          const smooth = t > c ? 0.5 : 0.1
          currentState.currentBlends[v] = c + (t - c) * smooth

          this.vrm.expressionManager.setValue(v, currentState.currentBlends[v])
        }
      }
    }

    state.animationId = requestAnimationFrame(animate)
  }

  private resetVowelBlendshapes(): void {
    if (this.vrm?.expressionManager) {
      for (const shape of VOWEL_SHAPES) {
        this.vrm.expressionManager.setValue(shape, 0)
      }
    }
  }
}

export default ChunkAnimationController
