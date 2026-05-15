import { createVRMAnimationClip, VRMAnimationLoaderPlugin } from '@pixiv/three-vrm-animation'
import type { VRMAnimation } from '@pixiv/three-vrm-animation'
import type { VRM } from '@pixiv/three-vrm'
import type { AnimationClip } from 'three'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js'

interface VrmAnimationAsset {
  animation: VRMAnimation
  source: string
}

export class VrmaLoader {
  private readonly animationCache = new Map<string, Promise<VrmAnimationAsset>>()

  async loadClip(source: string, vrm: VRM): Promise<AnimationClip> {
    const asset = await this.loadAnimation(source)
    return createVRMAnimationClip(asset.animation, vrm)
  }

  clear(): void {
    this.animationCache.clear()
  }

  private loadAnimation(source: string): Promise<VrmAnimationAsset> {
    const cached = this.animationCache.get(source)
    if (cached) {
      return cached
    }

    const pending = new Promise<VrmAnimationAsset>((resolve, reject) => {
      const loader = new GLTFLoader()
      loader.crossOrigin = 'anonymous'
      loader.register((parser) => new VRMAnimationLoaderPlugin(parser))
      loader.load(
        source,
        (gltf) => {
          const animations = (gltf.userData as { vrmAnimations?: VRMAnimation[] }).vrmAnimations
          const animation = animations?.[0]
          if (!animation) {
            reject(new Error(`No VRM animation found in ${source}`))
            return
          }

          resolve({ animation, source })
        },
        undefined,
        reject,
      )
    })

    this.animationCache.set(source, pending)
    return pending
  }
}
