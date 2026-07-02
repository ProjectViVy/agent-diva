import * as THREE from 'three'
import { PointerLockControls } from 'three/examples/jsm/controls/PointerLockControls.js'

/** Meters per second used as base movement speed, multiplied by delta time in update. */
const MOVE_SPEED = 5

/**
 * First-person WASD+QE movement controller backed by THREE.PointerLockControls.
 *
 * Designed to integrate with VrmRuntime for toggling between first-person free
 * movement and orbit-camera inspection. Keyboard movement processes on every
 * {@link update} tick using a per-frame delta for frame-rate-independent speed.
 *
 * Lifecycle:
 * 1. `attach()` – instantiate controls, register DOM listeners, add to scene.
 * 2. `update(deltaSeconds)` – process accumulated key state each frame.
 * 3. `detach()` – dispose controls, remove listeners, clean up scene.
 *
 * Lock state changes (including external ones like pressing Escape) are
 * detected via the `"pointerlockchange"` document event.
 */
export class PointerLockController {
  /** Underlying THREE.js first-person controls. Created during attach. */
  private controls: PointerLockControls | null = null

  /** Track currently held keys: `"KeyW" | "KeyA" | "KeyS" | "KeyD" | "KeyQ" | "KeyE"`. */
  private keyState: Record<string, boolean> = {}

  /** Whether the pointer is currently locked and keyboard movement is active. */
  private locked = false

  /** Bound event handlers stored so they can be cleanly removed in detach. */
  private readonly handlerKeyDown = (e: KeyboardEvent) => {
    this.keyState[e.code] = true
  }
  private readonly handlerKeyUp = (e: KeyboardEvent) => {
    this.keyState[e.code] = false
  }
  private readonly handlerPointerLockChange = () => {
    const wasLocked = this.locked
    this.locked = document.pointerLockElement === this.domElement
    // Re-sync key state when lock is lost externally (e.g. user presses Escape).
    if (wasLocked && !this.locked) {
      this.keyState = {}
    }
  }

  constructor(
    private readonly camera: THREE.PerspectiveCamera,
    private readonly domElement: HTMLCanvasElement,
    private readonly scene: THREE.Scene,
  ) {}

  // ── Lifecycle ────────────────────────────────────────────────────────

  /**
   * Create PointerLockControls, attach the head object to the scene,
   * and begin listening for keyboard + pointer-lock-change events.
   */
  attach(): void {
    if (this.controls) {
      console.warn('PointerLockController: already attached, skipping.')
      return
    }

    this.controls = new PointerLockControls(this.camera, this.domElement)
    this.controls.connect(this.domElement)
    this.scene.add(this.controls.object)

    document.addEventListener('keydown', this.handlerKeyDown)
    document.addEventListener('keyup', this.handlerKeyUp)
    document.addEventListener('pointerlockchange', this.handlerPointerLockChange)

    this.locked = document.pointerLockElement === this.domElement
  }

  /**
   * Exit pointer lock (if active), dispose controls, remove from scene,
   * and tear down all DOM listeners.
   */
  detach(): void {
    if (!this.controls) return

    if (this.locked) {
      this.controls.unlock()
    }
    this.locked = false
    this.keyState = {}

    this.scene.remove(this.controls.object)
    this.controls.disconnect()
    this.controls.dispose()
    this.controls = null

    document.removeEventListener('keydown', this.handlerKeyDown)
    document.removeEventListener('keyup', this.handlerKeyUp)
    document.removeEventListener('pointerlockchange', this.handlerPointerLockChange)
  }

  // ── Per-frame update ─────────────────────────────────────────────────

  /**
   * Apply WASD+QE movement based on current key-state and the given frame
   * delta. Calls {@link isLocked} internally – safe to call every frame even
   * when not locked.
   *
   * @param deltaSeconds Elapsed seconds since previous frame.
   */
  update(deltaSeconds: number): void {
    if (!this.controls || !this.locked) return

    const head = this.controls.object
    const direction = new THREE.Vector3()

    // Forward / back
    if (this.keyState['KeyW']) direction.z -= 1
    if (this.keyState['KeyS']) direction.z += 1
    // Left / right
    if (this.keyState['KeyA']) direction.x -= 1
    if (this.keyState['KeyD']) direction.x += 1
    // Down / up
    if (this.keyState['KeyQ']) direction.y -= 1
    if (this.keyState['KeyE']) direction.y += 1

    if (direction.lengthSq() === 0) return

    direction.normalize()
    direction.applyQuaternion(head.quaternion)
    head.position.addScaledVector(direction, MOVE_SPEED * deltaSeconds)
  }

  // ── Lock / unlock / toggle ───────────────────────────────────────────

  /** Request pointer lock on the canvas element and enable keyboard movement. */
  lock(): void {
    if (!this.controls) {
      console.warn('PointerLockController: cannot lock before attach().')
      return
    }
    this.controls.lock()
    this.locked = true
  }

  /** Exit pointer lock and disable keyboard movement. */
  unlock(): void {
    if (!this.controls) return
    this.controls.unlock()
    this.locked = false
    this.keyState = {}
  }

  /** Toggle between locked and unlocked states. */
  toggle(): void {
    if (this.locked) {
      this.unlock()
    } else {
      this.lock()
    }
  }

  /** Whether the pointer is currently captured and keyboard movement is active. */
  isLocked(): boolean {
    return this.locked
  }

  /**
   * Return a snapshot of the controller state suitable for bridge events
   * or external inspection.
   */
  getState(): { locked: boolean } {
    return { locked: this.locked }
  }
}

export default PointerLockController
