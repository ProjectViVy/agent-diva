import * as THREE from 'three'
import { SplatMesh } from '@sparkjsdev/spark'

export type GaussSceneId = 'transparent' | 'space' | 'home' | 'sea'

export interface GaussSceneConfig {
  source: GaussSceneId | string
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

function disposeObject(obj: THREE.Object3D): void {
  obj.traverse((child) => {
    if (child instanceof THREE.Mesh) {
      child.geometry?.dispose()
      const materials = Array.isArray(child.material)
        ? child.material
        : [child.material]
      for (const mat of materials) {
        disposeMaterial(mat)
      }
    }
    if (typeof (child as unknown as { dispose?: () => void }).dispose === 'function') {
      ;(child as unknown as { dispose(): void }).dispose()
    }
  })
}

export class GaussSceneController {
  private scene: THREE.Scene
  private currentGroup: THREE.Group | null = null
  private currentSceneId: GaussSceneId | null = null
  private splatMesh: SplatMesh | null = null

  constructor(scene: THREE.Scene) {
    this.scene = scene
  }

  async loadScene(
    sceneId: GaussSceneId | string,
    options?: { url?: string },
  ): Promise<void> {
    this.unloadCurrent()

    const group = new THREE.Group()
    group.name = `gaussScene_${sceneId}`

    if (sceneId === 'transparent') {
      const groundGeo = new THREE.PlaneGeometry(20, 20)
      const shadowMat = new THREE.ShadowMaterial({ opacity: 0.4 })
      const ground = new THREE.Mesh(groundGeo, shadowMat)
      ground.rotation.x = -Math.PI / 2
      ground.receiveShadow = true
      group.add(ground)
    } else {
      const sceneURL = options?.url ?? ''

      let splatHeight = 1.6
      let splatScale = 2

      if (sceneId === 'space') {
        splatHeight = 1.55
        splatScale = 2
      } else if (sceneId === 'home') {
        splatHeight = 1.6
        splatScale = 2
      } else if (sceneId === 'sea') {
        splatHeight = 2.4
        splatScale = 4
      }

      const splat = new SplatMesh({ url: sceneURL })
      await splat.initialized
      splat.quaternion.set(1, 0, 0, 0)
      splat.position.set(0, splatHeight, 2)
      splat.scale.set(splatScale, splatScale, splatScale)
      splat.receiveShadow = true
      this.splatMesh = splat
      group.add(splat)
    }

    this.scene.add(group)
    this.currentGroup = group
    this.currentSceneId = sceneId as GaussSceneId
  }

  private unloadCurrent(): void {
    if (this.currentGroup) {
      this.scene.remove(this.currentGroup)
      disposeObject(this.currentGroup)
      this.currentGroup = null
    }
    this.splatMesh = null
    this.currentSceneId = null
  }

  dispose(): void {
    this.unloadCurrent()
    this.scene = null as unknown as THREE.Scene
  }
}
