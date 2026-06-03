import type { VRM, VRMHumanBoneName } from '@pixiv/three-vrm'
import { VrmVersionAdapter } from './vrm-version-adapter'

// ─── VMC Protocol Types ───────────────────────────────────────────

export interface VmcBoneDatum {
  name: string
  pos: { x: number; y: number; z: number }
  rot: { x: number; y: number; z: number; w: number }
}

export interface VmcBlendDatum {
  name: string
  weight: number
}

export interface VmcFrame {
  bones: VmcBoneDatum[]
  blends: VmcBlendDatum[]
}

// ─── VMC Standard Bone List ───────────────────────────────────────

const VMC_BONES = [
  'hips', 'spine', 'chest', 'upperChest', 'neck', 'head',
  'leftShoulder', 'leftUpperArm', 'leftLowerArm', 'leftHand',
  'rightShoulder', 'rightUpperArm', 'rightLowerArm', 'rightHand',
  'leftUpperLeg', 'leftLowerLeg', 'leftFoot', 'leftToes',
  'rightUpperLeg', 'rightLowerLeg', 'rightFoot', 'rightToes',
  'leftThumbProximal', 'leftThumbIntermediate', 'leftThumbDistal',
  'leftIndexProximal', 'leftIndexIntermediate', 'leftIndexDistal',
  'leftMiddleProximal', 'leftMiddleIntermediate', 'leftMiddleDistal',
  'leftRingProximal', 'leftRingIntermediate', 'leftRingDistal',
  'leftLittleProximal', 'leftLittleIntermediate', 'leftLittleDistal',
  'rightThumbProximal', 'rightThumbIntermediate', 'rightThumbDistal',
  'rightIndexProximal', 'rightIndexIntermediate', 'rightIndexDistal',
  'rightMiddleProximal', 'rightMiddleIntermediate', 'rightMiddleDistal',
  'rightRingProximal', 'rightRingIntermediate', 'rightRingDistal',
  'rightLittleProximal', 'rightLittleIntermediate', 'rightLittleDistal',
] as const

// ─── VRM1 → VRM0 BlendShape Name Mapping (VMC de-facto standard) ──

const VRM1_TO_VMC0: Record<string, string> = {
  happy:      'Joy',
  angry:      'Angry',
  sad:        'Sorrow',
  relaxed:    'Fun',
  aa:         'A',
  ih:         'I',
  ou:         'U',
  ee:         'E',
  oh:         'O',
  blinkLeft:  'Blink_L',
  blinkRight: 'Blink_R',
  blink:      'Blink',
  surprised:  'Surprised',
  neutral:    'Neutral',
  lookDown:   'LookDown',
  lookUp:     'LookUp',
  lookLeft:   'LookLeft',
  lookRight:  'LookRight',
}

const VMC_BLEND_SHAPES = [
  'aa', 'ee', 'ih', 'oh', 'ou',
  'blink', 'blinkLeft', 'blinkRight',
  'surprised', 'happy', 'angry', 'sad', 'neutral', 'relaxed',
  'lookDown', 'lookUp', 'lookLeft', 'lookRight',
] as const

// ─── Controller ───────────────────────────────────────────────────

export class VmcController {
  private vrm: VRM | null = null
  private versionAdapter: VrmVersionAdapter | null = null

  attach(vrm: VRM): void {
    this.vrm = vrm
    this.versionAdapter = VrmVersionAdapter.detect(vrm)
  }

  detach(): void {
    this.vrm = null
    this.versionAdapter = null
  }

  /**
   * Extract bone position/rotation data in VMC protocol format.
   * Three.js (right-hand) → Unity (left-hand) coordinate conversion:
   *   Position: x → -x
   *   Rotation: y → -y, z → -z
   * For VRM0, input quaternion axes are additionally flipped before conversion.
   */
  getBoneData(): VmcBoneDatum[] {
    if (!this.vrm?.humanoid) {
      return []
    }

    const result: VmcBoneDatum[] = []

    for (const name of VMC_BONES) {
      const node = this.vrm.humanoid.getNormalizedBoneNode(name as VRMHumanBoneName)
      if (!node) {
        continue
      }

      // Position: Three.js right-hand → Unity left-hand (x negate)
      const vmcPos = {
        x: -node.position.x,
        y: node.position.y,
        z: node.position.z,
      }

      // Rotation quaternion conversion
      let qx = node.quaternion.x
      let qy = node.quaternion.y
      let qz = node.quaternion.z
      const qw = node.quaternion.w

      // VRM0 requires additional axis negation before converting to VMC
      const flipped = this.versionAdapter!.flipBoneQuaternion(qx, qy, qz, qw)

      // Three.js → Unity: y and z negated
      const vmcRot = {
        x: flipped.x,
        y: -flipped.y,
        z: -flipped.z,
        w: flipped.w,
      }

      result.push({ name, pos: vmcPos, rot: vmcRot })
    }

    return result
  }

  /**
   * Extract blend shape (expression) weights in VMC protocol format.
   * Maps VRM1 expression names to VMC0 standard names.
   */
  getBlendData(): VmcBlendDatum[] {
    if (!this.vrm?.expressionManager) {
      return []
    }

    const mgr = this.vrm.expressionManager
    const result: VmcBlendDatum[] = []

    for (const vrmName of VMC_BLEND_SHAPES) {
      const weight = mgr.getValue(vrmName)
      if (weight == null) {
        continue
      }

      const vmcName = VRM1_TO_VMC0[vrmName]
      if (!vmcName) {
        continue
      }

      result.push({ name: vmcName, weight })
    }

    return result
  }
}
