import type { VRM } from '@pixiv/three-vrm'
import * as THREE from 'three'

export type VrmVersion = '0' | '1'

export class VrmVersionAdapter {
  readonly version: VrmVersion

  constructor(version: VrmVersion) {
    this.version = version
  }

  static detect(vrm: VRM): VrmVersionAdapter {
    return new VrmVersionAdapter(vrm.meta?.metaVersion === '1' ? '1' : '0')
  }

  isVRM1(): boolean {
    return this.version === '1'
  }

  /**
   * For VRM0, negate x and z of the vector in-place (for lookat).
   * VRM1 leaves the vector unchanged.
   */
  flipViewVectorAxis(viewVector: THREE.Vector3): void {
    if (this.version === '0') {
      viewVector.z = -viewVector.z
      viewVector.x = -viewVector.x
    }
  }

  /**
   * For VRM0, negate qx and qz before VMC conversion.
   * VRM1 returns the quaternion unchanged.
   */
  flipBoneQuaternion(
    qx: number,
    qy: number,
    qz: number,
    qw: number,
  ): { x: number; y: number; z: number; w: number } {
    if (this.version === '0') {
      return { x: -qx, y: qy, z: -qz, w: qw }
    }
    return { x: qx, y: qy, z: qz, w: qw }
  }

  /**
   * Returns -1 for VRM1, 1 for VRM0.
   * Used for the pitch sign convention in lookat.
   */
  getLookAtPitchSign(): number {
    return this.version === '1' ? -1 : 1
  }
}
