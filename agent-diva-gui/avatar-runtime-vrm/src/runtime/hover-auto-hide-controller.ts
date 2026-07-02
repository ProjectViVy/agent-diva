import * as THREE from 'three'

/** Throttle interval for hover intersection checks in milliseconds (~30fps). */
const HOVER_CHECK_INTERVAL = 33

/** CSS opacity transition duration in milliseconds. */
const FADE_DURATION = 120

/**
 * Toggles VRM model visibility and canvas opacity when the mouse hovers
 * over the model mesh. Uses Three.js Raycaster for intersection testing
 * and CSS transitions for smooth fade effects.
 *
 * Lifecycle: attach → setEnabled(true) → per-frame update → detach
 *
 * @remarks
 * Mouse events are bound to `document` rather than the canvas element
 * because the canvas `pointerEvents` is set to `'none'` while hidden,
 * which would prevent mousemove detection needed to restore visibility.
 */
export class HoverAutoHideController {
  private renderer: THREE.WebGLRenderer | null = null
  private camera: THREE.PerspectiveCamera | null = null
  private vrmScene: THREE.Group | null = null

  private readonly raycaster = new THREE.Raycaster()
  private readonly mouse = new THREE.Vector2()

  private isEnabled = false
  private isHidden = false
  private hoverCheckTimeout: ReturnType<typeof setTimeout> | null = null
  private hideTransitionTimer: ReturnType<typeof setTimeout> | null = null

  // Pre-bound handlers so add/remove use the same function reference
  private readonly boundHandleMouseMove: (event: MouseEvent) => void
  private readonly boundHandleMouseLeave: (event: MouseEvent) => void

  constructor() {
    this.boundHandleMouseMove = this.handleMouseMove.bind(this)
    this.boundHandleMouseLeave = this.handleMouseLeave.bind(this)
  }

  // ─── Attach / Detach ──────────────────────────────────────────────

  /**
   * Binds the controller to a renderer, camera, and VRM scene group.
   * Registers document-level mouse listeners for hover detection.
   *
   * @param renderer - The Three.js WebGL renderer whose canvas is faded.
   * @param camera - The perspective camera used for raycasting.
   * @param vrmScene - The root THREE.Group of the loaded VRM model.
   */
  attach(
    renderer: THREE.WebGLRenderer,
    camera: THREE.PerspectiveCamera,
    vrmScene: THREE.Group,
  ): void {
    this.detach()
    this.renderer = renderer
    this.camera = camera
    this.vrmScene = vrmScene

    document.addEventListener('mousemove', this.boundHandleMouseMove)
    document.addEventListener('mouseleave', this.boundHandleMouseLeave)
  }

  /**
   * Removes all mouse listeners, clears pending timeouts, and resets
   * internal state. Call when the VRM model is unloaded or swapped.
   */
  detach(): void {
    document.removeEventListener('mousemove', this.boundHandleMouseMove)
    document.removeEventListener('mouseleave', this.boundHandleMouseLeave)

    if (this.hoverCheckTimeout) {
      clearTimeout(this.hoverCheckTimeout)
      this.hoverCheckTimeout = null
    }
    if (this.hideTransitionTimer) {
      clearTimeout(this.hideTransitionTimer)
      this.hideTransitionTimer = null
    }

    this.renderer = null
    this.camera = null
    this.vrmScene = null
    this.isHidden = false
  }

  // ─── Public API ───────────────────────────────────────────────────

  /**
   * Enables or disables the auto-hide feature. When disabled, any
   * currently-hidden model is immediately restored to visible.
   */
  setEnabled(enabled: boolean): void {
    this.isEnabled = enabled
    if (!enabled && this.isHidden) {
      this.showModel()
      this.isHidden = false
    }
  }

  /**
   * Returns a snapshot of the current controller state.
   *
   * @returns `{ enabled, hidden }` booleans.
   */
  getState(): { enabled: boolean; hidden: boolean } {
    return { enabled: this.isEnabled, hidden: this.isHidden }
  }

  /**
   * Per-frame update entry point. The hover detection is entirely
   * event-driven, so this method is a no-op unless future extensions
   * add per-frame work.
   *
   * @param _deltaSeconds - Elapsed seconds since last frame (unused).
   */
  update(_deltaSeconds?: number): void {
    // Event-driven; no per-frame work required
  }

  // ─── Event Handlers ───────────────────────────────────────────────

  /**
   * Throttled mousemove handler. Converts client coordinates to NDC,
   * casts a ray from the camera through the mouse position, and checks
   * for intersections with the VRM scene (recursive).
   *
   * Toggles visibility state when the hover status changes.
   */
  private handleMouseMove(event: MouseEvent): void {
    if (!this.vrmScene || !this.isEnabled || !this.renderer || !this.camera) {
      return
    }

    if (this.hoverCheckTimeout) {
      clearTimeout(this.hoverCheckTimeout)
    }

    this.hoverCheckTimeout = setTimeout(() => {
      if (!this.renderer || !this.camera || !this.vrmScene) return

      // Normalized Device Coordinates: [-1, 1]
      this.mouse.x = (event.clientX / window.innerWidth) * 2 - 1
      this.mouse.y = -(event.clientY / window.innerHeight) * 2 + 1

      this.raycaster.setFromCamera(this.mouse, this.camera)

      const intersects = this.raycaster.intersectObject(this.vrmScene, true)
      const nowHovered = intersects.length > 0

      if (nowHovered !== this.isHidden) {
        this.isHidden = nowHovered
        if (nowHovered) {
          this.hideModel()
        } else {
          this.showModel()
        }
      }
    }, HOVER_CHECK_INTERVAL)
  }

  /**
   * Fires when the mouse leaves the document entirely. If the model is
   * currently hidden, it is immediately restored to visible.
   */
  private handleMouseLeave(_event: MouseEvent): void {
    if (this.isEnabled && this.isHidden) {
      this.isHidden = false
      this.showModel()
    }
  }

  // ─── Show / Hide Transitions ──────────────────────────────────────

  /**
   * Hides the VRM model with a CSS opacity fade-out transition.
   *
   * 1. Applies `transition` on the renderer canvas.
   * 2. Uses `requestAnimationFrame` to trigger opacity → `'0'`, ensuring
   *    the browser recalculates the style before the transition fires.
   * 3. After `FADE_DURATION + 10ms`: disables pointer events on the
   *    canvas and sets `vrmScene.visible = false`.
   */
  private hideModel(): void {
    if (!this.renderer?.domElement) return
    const canvas = this.renderer.domElement as HTMLCanvasElement

    canvas.style.transition = `opacity ${FADE_DURATION}ms ease`

    if (this.hideTransitionTimer) {
      clearTimeout(this.hideTransitionTimer)
      this.hideTransitionTimer = null
    }

    requestAnimationFrame(() => {
      canvas.style.opacity = '0'
    })

    this.hideTransitionTimer = setTimeout(() => {
      canvas.style.pointerEvents = 'none'
      if (this.vrmScene) {
        this.vrmScene.visible = false
      }
      this.hideTransitionTimer = null
    }, FADE_DURATION + 10)
  }

  /**
   * Shows the VRM model with a CSS opacity fade-in transition.
   *
   * 1. Immediately sets opacity to `'0'` to establish the starting point.
   * 2. Applies `transition` and restores pointer events + scene visibility.
   * 3. Uses `requestAnimationFrame` to set opacity → `'1'`, which
   *    triggers the CSS transition on the next frame.
   */
  private showModel(): void {
    if (!this.renderer?.domElement) return
    const canvas = this.renderer.domElement as HTMLCanvasElement

    // Force current frame to 0 so transition reliably animates 0 → 1
    canvas.style.opacity = '0'
    canvas.style.transition = `opacity ${FADE_DURATION}ms ease`
    canvas.style.pointerEvents = 'auto'

    if (this.vrmScene) {
      this.vrmScene.visible = true
    }

    if (this.hideTransitionTimer) {
      clearTimeout(this.hideTransitionTimer)
      this.hideTransitionTimer = null
    }

    requestAnimationFrame(() => {
      canvas.style.opacity = '1'
    })
  }
}

export default HoverAutoHideController
