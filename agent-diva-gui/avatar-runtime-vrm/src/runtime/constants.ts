import type { AvatarTransform } from '../protocol'

export const RUNTIME_VERSION = '0.1.0'
export const DEFAULT_CAMERA_DISTANCE = 4.0
export const DEFAULT_CAMERA_TARGET_Y = 1.0

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
