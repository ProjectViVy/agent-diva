import type { AvatarInitOptions, AvatarRuntime, AvatarRuntimeHostBridge } from '../protocol'
import { RuntimeBridge } from './bridge'
import { VrmRuntime } from './vrm-runtime'

export type MountedVrmRuntime = AvatarRuntime & {
  bridge: AvatarRuntimeHostBridge
  setShadowEnabled(enabled: boolean): void
}

export async function createVrmRuntime(
  container: HTMLElement,
  options: AvatarInitOptions,
): Promise<MountedVrmRuntime> {
  const runtime = new VrmRuntime(container, options) as MountedVrmRuntime
  await runtime.init(options)
  return runtime
}

export { RuntimeBridge, VrmRuntime }
export { VrmVersionAdapter } from './vrm-version-adapter'
export type { VrmVersion } from './vrm-version-adapter'
export { PanoramaRenderer } from './panorama-renderer'
export { GaussSceneController } from './gauss-scene-controller'
export type { GaussSceneId, GaussSceneConfig } from './gauss-scene-controller'
export { LookAtController } from './lookat-controller'
export type { VmcBoneDatum, VmcBlendDatum, VmcFrame } from './vmc-controller'
export { VmcController } from './vmc-controller'
export { HoverAutoHideController } from './hover-auto-hide-controller'
export { PointerLockController } from './pointerlock-controller'
export { ChunkAnimationController } from './chunk-controller'
export type { ChunkControllerHooks } from './chunk-controller'
export { PttController } from './ptt-controller'
export type { PttControllerHooks } from './ptt-controller'
export { SubtitleController } from './subtitle-controller'
