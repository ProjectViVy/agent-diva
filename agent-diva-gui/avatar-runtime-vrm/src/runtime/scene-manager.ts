import * as THREE from 'three'
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js'
import { SparkRenderer } from '@sparkjsdev/spark'
import type { AvatarInitOptions, AvatarRuntimeMetrics, AvatarViewportSize } from '../protocol'
import {
  DEFAULT_CAMERA_DISTANCE,
  TRANSFORM_LIMITS,
  getCameraPosition,
  getCameraTarget,
  getDefaultTransform,
} from './constants'
import { GaussSceneController } from './gauss-scene-controller'
import type { GaussSceneId } from './gauss-scene-controller'
import { PanoramaRenderer } from './panorama-renderer'

type FrameHandler = (deltaSeconds: number) => void
type MetricsHandler = (metrics: AvatarRuntimeMetrics) => void

export class SceneManager {
  readonly scene = new THREE.Scene()
  readonly camera: THREE.PerspectiveCamera
  readonly renderer: THREE.WebGLRenderer
  readonly controls: OrbitControls

  private readonly timer = new THREE.Timer()
  private readonly frameHandlers = new Set<FrameHandler>()
  private animationFrameId: number | null = null
  private paused = true
  private destroyed = false
  private lastMetricsTime = 0
  private frameCount = 0
  private maxFps: number | null
  private onMetrics: MetricsHandler | null = null
  private gaussController: GaussSceneController | null = null
  private panorama: PanoramaRenderer | null = null
  private sparkRenderer: SparkRenderer | null = null

  private readonly keyLight: THREE.DirectionalLight
  private shadowGround: THREE.Mesh | null = null
  private shadowEnabled = false

  constructor(
    private readonly container: HTMLElement,
    options: AvatarInitOptions,
  ) {
    this.maxFps = options.maxFps ?? null

    this.renderer = new THREE.WebGLRenderer({
      alpha: options.transparent,
      antialias: true,
    })
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2))
    this.renderer.setClearAlpha(options.transparent ? 0 : 1)
    if (options.backgroundColor) {
      this.renderer.setClearColor(options.backgroundColor, options.transparent ? 0 : 1)
    }

    // ── Shadow Map ────────────────────────────────────────────────
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap

    const defaultTransform = getDefaultTransform(options.mode)

    this.camera = new THREE.PerspectiveCamera(30, 1, 0.1, 1000)
    this.camera.position.copy(getCameraPosition(defaultTransform))

    const ambientLight = new THREE.AmbientLight(0xffffff, 0.45)
    this.keyLight = new THREE.DirectionalLight(0xffffff, 0.9)
    this.keyLight.position.set(1, 2, 2)
    const fillLight = new THREE.DirectionalLight(0xffffff, 0.4)
    fillLight.position.set(-1.5, 1.5, 1)

    this.scene.add(ambientLight, this.keyLight, fillLight)

    this.sparkRenderer = new SparkRenderer({ renderer: this.renderer })
    this.scene.add(this.sparkRenderer)

    // ── Shadow Ground Plane ───────────────────────────────────────
    this.setShadowEnabled(!options.transparent)

    this.controls = new OrbitControls(this.camera, this.renderer.domElement)
    this.controls.target.copy(getCameraTarget(defaultTransform))
    this.controls.enablePan = false
    // Zoom distance range derived from scale limits (inverse: larger scale = closer camera).
    // scale.min=0.75 → maxDistance=3.2/0.75≈4.27, scale.max=1.6 → minDistance=3.2/1.6=2.0
    const zoomDistMin = DEFAULT_CAMERA_DISTANCE / TRANSFORM_LIMITS.scale.max
    const zoomDistMax = DEFAULT_CAMERA_DISTANCE / TRANSFORM_LIMITS.scale.min
    this.controls.enableZoom = false
    this.controls.minDistance = zoomDistMin
    this.controls.maxDistance = zoomDistMax
    this.controls.minPolarAngle = 0.85
    this.controls.maxPolarAngle = 2.2
    this.controls.enabled = options.allowInteraction
    this.controls.update()

    this.container.appendChild(this.renderer.domElement)
    this.resize({
      width: this.container.clientWidth,
      height: this.container.clientHeight,
    })
  }

  setInteractionEnabled(enabled: boolean): void {
    this.controls.enabled = enabled
  }

  setShadowEnabled(enabled: boolean): void {
    if (this.shadowEnabled === enabled) {
      return
    }
    this.shadowEnabled = enabled

    if (enabled) {
      this.renderer.shadowMap.enabled = true
      this.keyLight.castShadow = true
      this.keyLight.shadow.mapSize.set(2048, 2048)
      this.keyLight.shadow.camera.left = -4
      this.keyLight.shadow.camera.right = 4
      this.keyLight.shadow.camera.top = 4
      this.keyLight.shadow.camera.bottom = -4
      this.keyLight.shadow.camera.near = 0.1
      this.keyLight.shadow.camera.far = 20
      this.keyLight.shadow.bias = -0.0005

      if (!this.shadowGround) {
        const groundGeo = new THREE.PlaneGeometry(20, 20)
        const shadowMat = new THREE.ShadowMaterial({ opacity: 0.4 })
        this.shadowGround = new THREE.Mesh(groundGeo, shadowMat)
        this.shadowGround.rotation.x = -Math.PI / 2
        this.shadowGround.receiveShadow = true
      }
      this.shadowGround.visible = true
      this.scene.add(this.shadowGround)
    } else {
      this.renderer.shadowMap.enabled = false
      this.keyLight.castShadow = false
      if (this.shadowGround) {
        this.scene.remove(this.shadowGround)
      }
    }
  }

  onFrame(handler: FrameHandler): () => void {
    this.frameHandlers.add(handler)
    return () => this.frameHandlers.delete(handler)
  }

  setMetricsHandler(handler: MetricsHandler | null): void {
    this.onMetrics = handler
  }

  start(): void {
    if (this.destroyed || !this.paused) {
      return
    }

    this.paused = false
    this.timer.reset()
    this.timer.connect(document)
    this.lastMetricsTime = performance.now()
    this.frameCount = 0
    this.scheduleFrame()
  }

  pause(): void {
    this.paused = true
    if (this.animationFrameId !== null) {
      cancelAnimationFrame(this.animationFrameId)
      this.animationFrameId = null
    }
    this.timer.disconnect()
  }

  resume(): void {
    this.start()
  }

  setRenderMode(mode: 'normal' | 'panorama'): void {
    if (mode === 'panorama') {
      if (!this.panorama) {
        this.panorama = new PanoramaRenderer(this.renderer)
      }
      this.panorama.enable(new THREE.Vector3(0, 1.5, 1))
    } else {
      this.panorama?.disable()
    }
  }

  resize(size: AvatarViewportSize): void {
    const width = Math.max(1, Math.floor(size.width))
    const height = Math.max(1, Math.floor(size.height))
    this.camera.aspect = width / height
    this.camera.updateProjectionMatrix()
    this.renderer.setSize(width, height, false)
  }

  async setBackgroundScene(
    sceneId: GaussSceneId | string,
    url?: string,
  ): Promise<void> {
    if (!this.gaussController) {
      this.gaussController = new GaussSceneController(this.scene)
    }
    await this.gaussController.loadScene(sceneId, url ? { url } : undefined)
  }

  destroy(): void {
    this.destroyed = true
    this.pause()
    this.gaussController?.dispose()
    this.gaussController = null
    if (this.sparkRenderer) {
      this.scene.remove(this.sparkRenderer)
      this.sparkRenderer = null
    }
    this.controls.dispose()
    this.scene.clear()
    this.timer.dispose()
    this.renderer.dispose()
    this.renderer.domElement.remove()
  }

  private scheduleFrame(): void {
    if (this.paused || this.destroyed) {
      return
    }

    this.animationFrameId = requestAnimationFrame((timestamp) => {
      this.scheduleFrame()

      this.timer.update(timestamp)
      let delta = this.timer.getDelta()

      // Clamp delta to prevent animation teleporting when frames are
      // delayed (e.g. tab backgrounding or heavy GC pauses).  Without
      // clamping, mixer.update(largeDelta) skips multiple keyframes
      // causing visible position jumps — especially noticeable in
      // play_fingers / stretch VRMA animations.
      //
      // The old approach of SKIPPING frames entirely (returning early
      // when below maxFps threshold) caused delta values to accumulate
      // across skipped frames, producing even worse jumps.  We now
      // render every frame at native display rate and only cap the
      // animation delta, matching super-agent-party's smooth behavior.
      if (this.maxFps) {
        // Allow at most 1.5× the target frame interval for a single
        // update — any excess is deferred to subsequent frames so the
        // animation timeline stays accurate without visible popping.
        const maxDelta = (1.0 / this.maxFps) * 1.5
        delta = Math.min(delta, maxDelta)
      }

      this.controls.update()

      for (const handler of this.frameHandlers) {
        handler(delta)
      }

      if (this.panorama?.enabled) {
        this.panorama.render(this.scene)
      } else {
        this.renderer.render(this.scene, this.camera)
      }
      this.publishMetrics(timestamp, delta)
    })
  }

  private publishMetrics(timestamp: number, deltaSeconds: number): void {
    if (!this.onMetrics) {
      return
    }

    this.frameCount += 1
    const elapsed = timestamp - this.lastMetricsTime
    if (elapsed < 1000) {
      return
    }

    const metrics: AvatarRuntimeMetrics = {
      fps: Number(((this.frameCount * 1000) / elapsed).toFixed(1)),
      frameTimeMs: Number((deltaSeconds * 1000).toFixed(2)),
    }

    const memory = (performance as Performance & {
      memory?: { usedJSHeapSize: number }
    }).memory
    if (memory) {
      metrics.memoryMb = Number((memory.usedJSHeapSize / 1024 / 1024).toFixed(1))
    }

    this.lastMetricsTime = timestamp
    this.frameCount = 0
    this.onMetrics(metrics)
  }
}
