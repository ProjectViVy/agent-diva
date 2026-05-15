import * as THREE from 'three'
import type { Object3D } from 'three'
import type { AvatarTransform } from '../protocol'
import {
  DEFAULT_CAMERA_DISTANCE,
  DEFAULT_CAMERA_TARGET_Y,
  DEFAULT_TRANSFORM,
  TRANSFORM_LIMITS,
} from './constants'

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max)
}

export class TransformController {
  private modelRoot: Object3D | null = null
  private current: AvatarTransform = { ...DEFAULT_TRANSFORM }

  constructor(
    private readonly camera: THREE.PerspectiveCamera,
    private readonly controls: {
      target: THREE.Vector3
      update(): void
      getAzimuthalAngle(): number
      getPolarAngle(): number
    },
  ) {}

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
    this.current = { ...DEFAULT_TRANSFORM }
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
      this.modelRoot.position.set(this.current.offsetX, this.current.offsetY, 0)
    }

    this.controls.target.set(
      this.current.offsetX,
      DEFAULT_CAMERA_TARGET_Y + this.current.offsetY,
      0,
    )

    // Map scale to camera distance (inverse relationship: larger scale = closer camera)
    // This mirrors how ChatVRM uses OrbitControls camera-distance zoom.
    const effectiveDistance = DEFAULT_CAMERA_DISTANCE / this.current.scale

    const offset = new THREE.Vector3().setFromSphericalCoords(
      effectiveDistance,
      this.current.rotationPolar,
      this.current.rotationAzimuth,
    )
    this.camera.position.copy(this.controls.target).add(offset)
    // NOTE: do NOT call controls.update() here — its internal spherical state
    // would overwrite the camera.position we just set. The animation loop
    // (SceneManager.scheduleFrame) calls controls.update() every frame, which
    // re-reads camera.position and reconciles its internal state.
  }
}
