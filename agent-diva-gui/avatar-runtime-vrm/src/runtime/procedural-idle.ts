import type { VRM, VRMHumanBoneName } from '@pixiv/three-vrm'
import {
  AnimationClip,
  Euler,
  KeyframeTrack,
  Quaternion,
  QuaternionKeyframeTrack,
} from 'three'

const idleOffsets = {
  body: Math.PI * 0.13,
  leftArm: Math.PI * 0.41,
  rightArm: Math.PI * 0.73,
  head: Math.PI * 1.1,
  leftShoulder: Math.PI * 1.37,
  rightShoulder: Math.PI * 1.62,
}

const IDLE_BONES = [
  'spine',
  'chest',
  'neck',
  'head',
  'leftUpperArm',
  'leftLowerArm',
  'leftHand',
  'leftShoulder',
  'rightUpperArm',
  'rightLowerArm',
  'rightHand',
  'rightShoulder',
] as const

const FINGER_BONES = [
  'leftThumbProximal',
  'leftThumbIntermediate',
  'leftThumbDistal',
  'leftIndexProximal',
  'leftIndexIntermediate',
  'leftIndexDistal',
  'leftMiddleProximal',
  'leftMiddleIntermediate',
  'leftMiddleDistal',
  'leftRingProximal',
  'leftRingIntermediate',
  'leftRingDistal',
  'leftLittleProximal',
  'leftLittleIntermediate',
  'leftLittleDistal',
  'rightThumbProximal',
  'rightThumbIntermediate',
  'rightThumbDistal',
  'rightIndexProximal',
  'rightIndexIntermediate',
  'rightIndexDistal',
  'rightMiddleProximal',
  'rightMiddleIntermediate',
  'rightMiddleDistal',
  'rightRingProximal',
  'rightRingIntermediate',
  'rightRingDistal',
  'rightLittleProximal',
  'rightLittleIntermediate',
  'rightLittleDistal',
] as const

interface BonePose {
  x: number
  y: number
  z: number
}

function getNaturalPose(boneName: string, isVrm1: boolean): BonePose {
  const v = isVrm1 ? 1 : -1
  switch (boneName) {
    case 'leftUpperArm':
      return { x: 0.05, y: 0, z: -0.45 * Math.PI * v }
    case 'rightUpperArm':
      return { x: 0.05, y: 0, z: 0.45 * Math.PI * v }
    case 'leftHand':
      return { x: 0.05, y: 0, z: 0.1 * v }
    case 'rightHand':
      return { x: 0.05, y: 0, z: -0.1 * v }
    case 'leftUpperLeg':
      return { x: 0, y: 0.05 * v, z: 0.04 * v }
    case 'rightUpperLeg':
      return { x: 0, y: -0.05 * v, z: -0.04 * v }
    default:
      return { x: 0, y: 0, z: 0 }
  }
}

function applyFingerCurl(vrm: VRM, isVrm1: boolean): void {
  const v = isVrm1 ? 1 : -1
  for (const boneName of FINGER_BONES) {
    const bone = vrm.humanoid?.getNormalizedBoneNode(boneName as VRMHumanBoneName)
    if (!bone) {
      continue
    }

    const euler = new Euler(0, 0, 0)
    if (boneName.includes('Thumb')) {
      euler.y = boneName.includes('left') ? 0.35 : -0.35
    } else if (boneName.includes('Proximal')) {
      euler.z = boneName.includes('left') ? -0.35 * v : 0.35 * v
    } else if (boneName.includes('Intermediate')) {
      euler.z = boneName.includes('left') ? -0.45 * v : 0.45 * v
    } else if (boneName.includes('Distal')) {
      euler.z = boneName.includes('left') ? -0.3 * v : 0.3 * v
    }
    bone.quaternion.setFromEuler(euler)
  }
}

export function applyNaturalPose(vrm: VRM): void {
  if (!vrm.humanoid) {
    return
  }

  const isVrm1 = vrm.meta?.metaVersion === '1'
  for (const boneName of [
    'leftUpperArm',
    'rightUpperArm',
    'leftHand',
    'rightHand',
    'leftUpperLeg',
    'rightUpperLeg',
  ]) {
    const bone = vrm.humanoid.getNormalizedBoneNode(boneName as VRMHumanBoneName)
    if (!bone) {
      continue
    }

    const pose = getNaturalPose(boneName, isVrm1)
    bone.quaternion.setFromEuler(new Euler(pose.x, pose.y, pose.z))
  }

  applyFingerCurl(vrm, isVrm1)
}

export function createProceduralIdleClip(vrm: VRM): AnimationClip | null {
  if (!vrm.humanoid) {
    return null
  }

  const fps = 30
  const duration = 30
  const frameCount = duration * fps
  const isVrm1 = vrm.meta?.metaVersion === '1'
  const v = isVrm1 ? 1 : -1
  const times: number[] = []

  for (let index = 0; index <= frameCount; index += 1) {
    times.push(index / fps)
  }

  const tracks: KeyframeTrack[] = []

  for (const boneName of IDLE_BONES) {
    const bone = vrm.humanoid.getNormalizedBoneNode(boneName as VRMHumanBoneName)
    if (!bone) {
      continue
    }

    const values: number[] = []
    for (const time of times) {
      const cycleTime = (time / duration) * Math.PI * 2
      const euler = new Euler(0, 0, 0)

      switch (boneName) {
        case 'spine':
          euler.set(
            Math.sin(cycleTime + idleOffsets.body) * 0.015,
            0,
            Math.cos(cycleTime * 1.3 + idleOffsets.body) * 0.01,
          )
          break
        case 'chest':
          euler.set(
            Math.sin(cycleTime + idleOffsets.body) * 0.008,
            0,
            Math.cos(cycleTime * 1.3 + idleOffsets.body) * 0.005,
          )
          break
        case 'neck':
          euler.set(
            Math.cos(cycleTime * 2.4 + idleOffsets.head) * 0.01,
            Math.sin(cycleTime * 3 + idleOffsets.head) * 0.015,
            0,
          )
          break
        case 'head':
          euler.set(
            Math.sin(cycleTime * 2 + idleOffsets.head) * 0.015,
            Math.sin(cycleTime * 2.5 + idleOffsets.head) * 0.02,
            Math.cos(cycleTime * 1.6 + idleOffsets.head) * 0.008,
          )
          break
        case 'leftUpperArm':
          euler.set(
            Math.cos(cycleTime * 1.4 + idleOffsets.leftArm) * 0.02,
            Math.sin(cycleTime * 1.2 + idleOffsets.leftArm) * 0.015,
            -0.45 * Math.PI * v + Math.sin(cycleTime * 2 + idleOffsets.leftArm) * 0.02,
          )
          break
        case 'leftLowerArm':
          euler.set(0, 0, -Math.sin(cycleTime * 2 + idleOffsets.leftArm) * 0.015)
          break
        case 'leftHand':
          euler.set(0.05, 0, 0.1 * v + Math.sin(cycleTime * 2 + idleOffsets.leftArm) * 0.01)
          break
        case 'leftShoulder':
          euler.set(0, 0, Math.sin(cycleTime * 1.4 + idleOffsets.leftShoulder) * 0.015)
          break
        case 'rightUpperArm':
          euler.set(
            Math.cos(cycleTime * 1.6 + idleOffsets.rightArm) * 0.02,
            Math.sin(cycleTime * 1.28 + idleOffsets.rightArm) * 0.015,
            0.45 * Math.PI * v + Math.sin(cycleTime * 2 + idleOffsets.rightArm) * 0.02,
          )
          break
        case 'rightLowerArm':
          euler.set(0, 0, Math.sin(cycleTime * 2 + idleOffsets.rightArm) * 0.015)
          break
        case 'rightHand':
          euler.set(0.05, 0, -0.1 * v + Math.sin(cycleTime * 2 + idleOffsets.rightArm) * 0.01)
          break
        case 'rightShoulder':
          euler.set(0, 0, Math.sin(cycleTime * 1.6 + idleOffsets.rightShoulder) * 0.015)
          break
      }

      const quat = new Quaternion().setFromEuler(euler)
      values.push(quat.x, quat.y, quat.z, quat.w)
    }

    tracks.push(new QuaternionKeyframeTrack(`${bone.name}.quaternion`, times, values))
  }

  return new AnimationClip('idle', duration, tracks)
}
