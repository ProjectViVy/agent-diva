import type { VRM } from '@pixiv/three-vrm'
import * as THREE from 'three'
import { VrmVersionAdapter } from './vrm-version-adapter'

export class LookAtController {
  private vrm: VRM | null = null
  private camera: THREE.PerspectiveCamera | null = null
  private versionAdapter: VrmVersionAdapter | null = null

  private currentYaw = 0
  private currentPitch = 0

  // ─── Tweakable Limits (radians) ─────────────────────────────────

  /** Max left/right head turn in radians (default 45°) */
  yawLimit = THREE.MathUtils.degToRad(45)
  /** Max upward head tilt in radians (default 40°) */
  pitchUpLimit = THREE.MathUtils.degToRad(40)
  /** Max downward head tilt in radians (default 20°) */
  pitchDownLimit = THREE.MathUtils.degToRad(20)
  /** Head stops tracking when camera is behind this angle (default 110°) */
  behindAngleLimit = THREE.MathUtils.degToRad(110)

  /** How much of the total yaw/pitch goes to the neck (0-1). Remainder goes to head. */
  neckRatio = 1.0
  headRatio = 0.5

  /** Lerp smoothing speed. Higher = faster tracking. */
  lerpSpeed = 3.0

  // ─── Attach / Detach ────────────────────────────────────────────

  attach(vrm: VRM, camera: THREE.PerspectiveCamera): void {
    this.vrm = vrm
    this.camera = camera
    this.versionAdapter = VrmVersionAdapter.detect(vrm)
    this.currentYaw = 0
    this.currentPitch = 0
  }

  detach(): void {
    this.vrm = null
    this.camera = null
    this.versionAdapter = null
    this.currentYaw = 0
    this.currentPitch = 0
  }

  // ─── Per-frame Update ───────────────────────────────────────────

  /**
   * Call every frame BEFORE vrm.update(delta).
   * Computes view vector from neck bone to camera, applies limits,
   * lerps smoothly, and sets neck+head bone quaternions.
   */
  update(deltaSeconds: number): void {
    if (!this.vrm?.humanoid || !this.camera) {
      return
    }

    const neck = this.vrm.humanoid.getNormalizedBoneNode('neck')
    const head = this.vrm.humanoid.getNormalizedBoneNode('head')
    if (!neck?.parent) {
      return
    }

    // ── Step 1: Compute view vector in neck-parent local space ─────
    const parent = neck.parent
    const targetWorldPos = this.camera.position.clone()
    const localCameraPos = parent.worldToLocal(targetWorldPos)
    const neckLocalPos = neck.position.clone()
    const viewVector = localCameraPos.sub(neckLocalPos)

    // VRM0 requires axis negation
    this.versionAdapter!.flipViewVectorAxis(viewVector)

    // ── Step 2: Compute target yaw/pitch from view vector ──────────
    const rawTargetYaw = Math.atan2(viewVector.x, viewVector.z)
    const horizontalDist = Math.sqrt(viewVector.x ** 2 + viewVector.z ** 2)
    const rawTargetPitch = Math.atan2(viewVector.y, horizontalDist)

    let targetYaw = rawTargetYaw * 0.6
    let targetPitch = rawTargetPitch * 0.6

    // Behind-angle check: if camera is too far behind, stop tracking
    if (Math.abs(rawTargetYaw) > this.behindAngleLimit) {
      targetYaw = 0
      targetPitch = 0
    } else {
      targetYaw = THREE.MathUtils.clamp(targetYaw, -this.yawLimit, this.yawLimit)
      targetPitch = THREE.MathUtils.clamp(
        targetPitch,
        -this.pitchDownLimit,
        this.pitchUpLimit,
      )
    }

    // ── Step 3: Smooth lerp ───────────────────────────────────────
    const lerpAmount = this.lerpSpeed * deltaSeconds
    this.currentYaw = THREE.MathUtils.lerp(this.currentYaw, targetYaw, lerpAmount)
    this.currentPitch = THREE.MathUtils.lerp(this.currentPitch, targetPitch, lerpAmount)

    // ── Step 4: Apply to bones ────────────────────────────────────
    const pitchSign = this.versionAdapter!.getLookAtPitchSign()
    const applyYaw = this.currentYaw
    const applyPitch = pitchSign * this.currentPitch

    // Neck (primary rotation)
    const neckQ = new THREE.Quaternion()
      .multiply(new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), applyYaw * this.neckRatio))
      .multiply(new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(1, 0, 0), applyPitch * this.neckRatio))
    neck.quaternion.copy(neckQ)

    // Head (secondary, additive rotation for more natural look)
    if (head) {
      const headQ = new THREE.Quaternion()
        .multiply(new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(0, 1, 0), applyYaw * this.headRatio))
        .multiply(new THREE.Quaternion().setFromAxisAngle(new THREE.Vector3(1, 0, 0), applyPitch * this.headRatio))
      head.quaternion.copy(headQ)
    }
  }
}
