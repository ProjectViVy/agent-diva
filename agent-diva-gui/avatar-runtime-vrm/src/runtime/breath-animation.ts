import type { VRM } from '@pixiv/three-vrm'
import { AnimationClip, VectorKeyframeTrack } from 'three'

export function createBreathClip(_vrm: VRM): AnimationClip {
  const duration = 4
  const fps = 30
  const frameCount = duration * fps
  const times: number[] = []
  const scaleValues: number[] = []

  for (let index = 0; index <= frameCount; index += 1) {
    const time = index / fps
    times.push(time)
    const breathScale = 1 + Math.sin((time * Math.PI) / 2) * 0.005
    scaleValues.push(breathScale, breathScale, breathScale)
  }

  return new AnimationClip('breath', duration, [
    new VectorKeyframeTrack('.scale', times, scaleValues),
  ])
}
