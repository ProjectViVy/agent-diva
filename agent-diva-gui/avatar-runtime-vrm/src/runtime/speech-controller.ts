import type { VRM } from '@pixiv/three-vrm'
import type { AvatarSpeechState } from '../protocol'

const VISEME_SHAPES = ['aa', 'ih', 'ou', 'ee', 'oh'] as const

export class SpeechController {
  private vrm: VRM | null = null
  private state: AvatarSpeechState = { speaking: false }
  private phase = 0

  attach(vrm: VRM): void {
    this.vrm = vrm
    this.reset()
  }

  detach(): void {
    this.reset()
    this.vrm = null
  }

  setState(state: AvatarSpeechState): void {
    this.state = {
      speaking: state.speaking,
      viseme: state.viseme ?? null,
      intensity: state.intensity ?? 0.8,
    }

    if (!state.speaking) {
      this.reset()
    }
  }

  update(deltaSeconds: number): void {
    if (!this.vrm?.expressionManager || !this.state.speaking) {
      return
    }

    this.phase += deltaSeconds * 7
    const intensity = Math.min(Math.max(this.state.intensity ?? 0.8, 0), 1)
    const animatedValue = 0.2 + Math.abs(Math.sin(this.phase)) * 0.8 * intensity
    const requested = this.state.viseme ?? null
    const activeShape = VISEME_SHAPES.includes(requested as (typeof VISEME_SHAPES)[number])
      ? requested
      : VISEME_SHAPES[Math.floor(this.phase) % VISEME_SHAPES.length]

    for (const shape of VISEME_SHAPES) {
      this.vrm.expressionManager.setValue(shape, shape === activeShape ? animatedValue : 0)
    }
  }

  reset(): void {
    if (this.vrm?.expressionManager) {
      for (const shape of VISEME_SHAPES) {
        this.vrm.expressionManager.setValue(shape, 0)
      }
    }
    this.phase = 0
  }
}
