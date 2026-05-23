import type { AvatarTransform } from '../protocol'
import type { AvatarRuntimeMode } from '@morediva/shared-avatar-protocol'
import * as THREE from 'three'

export const RUNTIME_VERSION = '0.1.0'
export const DEFAULT_CAMERA_DISTANCE = 3.0
export const DEFAULT_CAMERA_TARGET_X = 0.3
export const DEFAULT_CAMERA_TARGET_Y = 0.6

export const TRANSFORM_LIMITS = {
  scale: { min: 0.75, max: 1.6 },
  offsetX: { min: -1.2, max: 1.2 },
  offsetY: { min: -1.0, max: 1.2 },
  rotationPolar: { min: 0.85, max: 2.2 },
} as const

export const DEFAULT_TRANSFORM: AvatarTransform = {
  scale: 1,
  offsetX: 0,
  offsetY: 0,
  rotationAzimuth: 0,
  rotationPolar: 1.089,
}

export const DESKTOP_PET_DEFAULT_TRANSFORM: AvatarTransform = {
  scale: 1,
  offsetX: 0,
  offsetY: 0,
  rotationAzimuth: 0,
  rotationPolar: Math.PI / 2,
}

export function getDefaultTransform(mode: AvatarRuntimeMode): AvatarTransform {
  return {
    ...(mode === 'desktop-pet' ? DESKTOP_PET_DEFAULT_TRANSFORM : DEFAULT_TRANSFORM),
  }
}

export function getCameraTarget(transform: AvatarTransform): THREE.Vector3 {
  return new THREE.Vector3(
    DEFAULT_CAMERA_TARGET_X + transform.offsetX,
    DEFAULT_CAMERA_TARGET_Y + transform.offsetY,
    0,
  )
}

export function getCameraPosition(transform: AvatarTransform): THREE.Vector3 {
  const target = getCameraTarget(transform)
  const effectiveDistance = DEFAULT_CAMERA_DISTANCE / transform.scale
  const offset = new THREE.Vector3().setFromSphericalCoords(
    effectiveDistance,
    transform.rotationPolar,
    transform.rotationAzimuth,
  )
  return target.add(offset)
}

export const DEFAULT_CAPABILITIES = [
  'vrm',
  'transparent-background',
  'rotate',
  'transform',
  'mood',
  'speech',
  'base-animation',
  'vrma-motion',
  'vmc-send',
  'bone-lookat',
  'shadow',
  'command-dispatch',
  'vrm-version-adapter',
  'gauss-scene',
  'panorama-render',
  'hover-auto-hide',
  'pointerlock',
  'chunk-animation',
  'push-to-talk',
  'subtitle-overlay',
] as const
