import * as THREE from 'three'
import type { Object3D } from 'three'
import type { AvatarRuntimeMode } from '@morediva/shared-avatar-protocol'
import type { AvatarTransform } from '../protocol'
import {
  TRANSFORM_LIMITS,
  getCameraPosition,
  getCameraTarget,
  getDefaultTransform,
} from './constants'

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max)
}

export class TransformController {
  private modelRoot: Object3D | null = null
  private readonly initial: AvatarTransform
  private current: AvatarTransform

  constructor(
    private readonly camera: THREE.PerspectiveCamera,
    private readonly controls: {
      target: THREE.Vector3
      update(): void
      getAzimuthalAngle(): number
      getPolarAngle(): number
    },
    mode: AvatarRuntimeMode,
  ) {
    this.initial = getDefaultTransform(mode)
    this.current = { ...this.initial }
  }

  attachModel(root: Object3D | null): void {
    this.modelRoot = root
    this.applyCurrent()
  }

  setTransform(next: Partial<AvatarTransform>): AvatarTransform {
    this.current = this.normalize({
      ...this.current,
      ...next,
    })
    this.applyCurrent()
    return this.getTransform()
  }

  syncFromInteraction(): AvatarTransform {
    this.current = this.normalize({
      ...this.current,
      rotationAzimuth: this.controls.getAzimuthalAngle(),
      rotationPolar: this.controls.getPolarAngle(),
    })
    return this.getTransform()
  }

  getTransform(): AvatarTransform {
    return { ...this.current }
  }

  reset(): AvatarTransform {
    this.current = { ...this.initial }
    this.applyCurrent()
    return this.getTransform()
  }

  private normalize(transform: AvatarTransform): AvatarTransform {
    return {
      scale: clamp(transform.scale, TRANSFORM_LIMITS.scale.min, TRANSFORM_LIMITS.scale.max),
      offsetX: clamp(transform.offsetX, TRANSFORM_LIMITS.offsetX.min, TRANSFORM_LIMITS.offsetX.max),
      offsetY: clamp(transform.offsetY, TRANSFORM_LIMITS.offsetY.min, TRANSFORM_LIMITS.offsetY.max),
      rotationAzimuth: transform.rotationAzimuth,
      rotationPolar: clamp(
        transform.rotationPolar,
        TRANSFORM_LIMITS.rotationPolar.min,
        TRANSFORM_LIMITS.rotationPolar.max,
      ),
    }
  }

  private applyCurrent(): void {
    if (this.modelRoot) {
      this.modelRoot.position.set(0, 0, 0)
    }

    this.controls.target.copy(getCameraTarget(this.current))
    this.camera.position.copy(getCameraPosition(this.current))
    // Keep OrbitControls' spherical state aligned with the programmatic camera preset.
    this.controls.update()
  }
}
