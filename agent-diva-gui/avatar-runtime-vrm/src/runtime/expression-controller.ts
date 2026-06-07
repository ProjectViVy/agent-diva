import type { VRM } from '@pixiv/three-vrm'
import type { AvatarMood } from '../protocol'

const MOODS: AvatarMood[] = ['neutral', 'happy', 'sad', 'angry', 'surprised']

export class ExpressionController {
  private vrm: VRM | null = null
  private mood: AvatarMood = 'neutral'

  attach(vrm: VRM): void {
    this.vrm = vrm
    this.applyMood(this.mood)
  }

  detach(): void {
    this.vrm = null
  }

  setMood(mood: AvatarMood): void {
    this.mood = mood
    this.applyMood(mood)
  }

  getMood(): AvatarMood {
    return this.mood
  }

  private applyMood(mood: AvatarMood): void {
    if (!this.vrm?.expressionManager) {
      return
    }

    for (const current of MOODS) {
      const value = mood === 'neutral' ? 0 : current === mood ? 1 : 0
      this.vrm.expressionManager.setValue(current, value)
    }
  }
}
