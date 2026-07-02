import type { VRM } from '@pixiv/three-vrm'
import { AnimationClip, NumberKeyframeTrack } from 'three'

export function createBlinkClip(vrm: VRM): AnimationClip | null {
  if (!vrm.expressionManager) {
    return null
  }

  const duration = 6
  const fps = 30
  const frameCount = duration * fps
  const times: number[] = []
  const blinkValues: number[] = []

  for (let index = 0; index <= frameCount; index += 1) {
    const time = index / fps
    times.push(time)

    let blinkValue = 0
    if (time >= 1.5 && time <= 1.7) {
      blinkValue = Math.sin(((time - 1.5) / 0.2) * Math.PI)
    } else if (time >= 3.8 && time <= 4.4) {
      const localTime = time - 3.8
      if (localTime < 0.15) {
        blinkValue = Math.sin((localTime / 0.15) * Math.PI)
      } else if (localTime > 0.25 && localTime < 0.4) {
        blinkValue = Math.sin(((localTime - 0.25) / 0.15) * Math.PI)
      }
    }

    blinkValues.push(blinkValue)
  }

  const trackName = vrm.expressionManager.getExpressionTrackName('blink')
  if (!trackName) {
    return null
  }

  return new AnimationClip('blink', duration, [
    new NumberKeyframeTrack(trackName, times, blinkValues),
  ])
}
