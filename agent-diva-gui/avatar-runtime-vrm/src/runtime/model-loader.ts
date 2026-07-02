import * as THREE from 'three'
import { VRMLoaderPlugin, VRMUtils, type VRM } from '@pixiv/three-vrm'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js'
import { applyNaturalPose } from './procedural-idle'

function normalizeMaterials(root: THREE.Object3D): void {
  root.traverse((object) => {
    if (!(object instanceof THREE.Mesh) || !object.material) {
      return
    }

    const materials = Array.isArray(object.material) ? object.material : [object.material]
    for (const material of materials) {
      const mutable = material as THREE.Material & {
        alphaTest?: number
        depthWrite?: boolean
        blending?: THREE.Blending
        premultipliedAlpha?: boolean
        transparent?: boolean
        needsUpdate?: boolean
      }

      if (mutable.transparent) {
        mutable.alphaTest = 0.01
        mutable.depthWrite = true
      }

      mutable.blending = THREE.NormalBlending
      mutable.premultipliedAlpha = false
      mutable.needsUpdate = true
    }
  })
}

function disposeMaterial(material: THREE.Material): void {
  for (const key of Object.keys(material)) {
    const value = (material as unknown as Record<string, unknown>)[key]
    if (value instanceof THREE.Texture) {
      value.dispose()
    }
  }
  material.dispose()
}

export class VrmModelLoader {
  async load(modelSource: string, camera: THREE.Camera): Promise<VRM> {
    const loader = new GLTFLoader()
    loader.crossOrigin = 'anonymous'
    loader.register((parser) => new VRMLoaderPlugin(parser))

    const gltf = await new Promise<{ scene: THREE.Group; userData: Record<string, unknown> }>((resolve, reject) => {
      loader.load(modelSource, resolve, undefined, reject)
    })

    try {
      VRMUtils.removeUnnecessaryVertices(gltf.scene)
    } catch {
      // three-vrm optimization helper is optional across model versions.
    }
    try {
      VRMUtils.combineSkeletons(gltf.scene)
    } catch {
      // Some models fail this optimization; keep loading instead.
    }

    const vrm = gltf.userData.vrm as VRM | undefined
    if (!vrm) {
      throw new Error('VRM model not found in loaded GLTF data')
    }

    VRMUtils.rotateVRM0(vrm)
    try {
      VRMUtils.combineMorphs(vrm)
    } catch {
      // Morph optimization is not required for phase 1 runtime correctness.
    }

    normalizeMaterials(gltf.scene)
    gltf.scene.traverse((object) => {
      object.frustumCulled = false
      if (object instanceof THREE.Mesh) {
        object.castShadow = true
        object.receiveShadow = true
      }
    })

    vrm.expressionManager?.setValue('neutral', 1)
    applyNaturalPose(vrm)

    if (vrm.lookAt) {
      vrm.lookAt.target = camera
      if (vrm.lookAt.applier) {
        const applier = vrm.lookAt.applier as { yawLimit?: number; pitchLimit?: number }
        applier.yawLimit = 60
        applier.pitchLimit = 30
      }
    }

    return vrm
  }

  dispose(vrm: VRM | null): void {
    if (!vrm) {
      return
    }

    if (typeof (vrm as unknown as { dispose?: () => void }).dispose === 'function') {
      ;(vrm as unknown as { dispose(): void }).dispose()
    }

    vrm.scene.traverse((object) => {
      if (!(object instanceof THREE.Mesh)) {
        return
      }

      object.geometry?.dispose()
      const materials = Array.isArray(object.material) ? object.material : [object.material]
      for (const material of materials) {
        disposeMaterial(material)
      }
    })
  }
}
