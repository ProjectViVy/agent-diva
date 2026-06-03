/**
 * useLookAt — Vue 3 composable for Phase 4 Look-at Tracking.
 *
 * Wraps {@link LookAtController} and provides:
 *   - Reactive enable/disable of gaze tracking
 *   - Target position management (camera, mouse, or fixed world point)
 *   - Per-frame update loop (via requestAnimationFrame)
 *   - Graceful degradation when the crate does not expose the look-at API
 *
 * Reference: super-agent-party vrm.js:2153-2213
 *
 * Integration: Attempts to set look-at target on the runtime when
 * available; otherwise runs as a pure calculation engine.
 *
 * Pattern: Follows useVoicePlayer / useLipSync conventions
 *   - Options interface for input
 *   - Result interface for output
 *   - ref() / shallowRef() for reactivity
 *   - onUnmounted() cleanup
 */

import {
    onUnmounted,
    ref,
    shallowRef,
    watch,
    type Ref,
    type ShallowRef,
} from 'vue'
import type { VrmLookAtTarget } from '../types'
import { addVoiceLogEvent } from '../voice/services/voice-log'
import {
    LookAtController,
    type LookAtConfig,
    type LookAtState,
} from './look-at-controller'

// ── Options ──────────────────────────────────────────────────────

export interface UseLookAtOptions {
    /** Whether look-at tracking is enabled globally */
    enabled?: Ref<boolean> | boolean
    /** Initial target position */
    target?: Ref<VrmLookAtTarget> | VrmLookAtTarget
    /** Controller configuration overrides */
    config?: Partial<LookAtConfig>
    /** Update rate (calls per second). 0 = run on every rAF frame. */
    updateRate?: number
}

// ── Result ───────────────────────────────────────────────────────

export interface UseLookAtResult {
    /** The underlying LookAtController instance */
    controller: ShallowRef<LookAtController | null>

    /** Current look-at state (reactive snapshot) */
    state: Ref<LookAtState>

    /** Whether tracking is currently enabled */
    isEnabled: Ref<boolean>

    /** Whether a valid target is being tracked this frame */
    isTracking: Ref<boolean>

    /** Current yaw angle in degrees (readonly) */
    yawDeg: Ref<number>

    /** Current pitch angle in degrees (readonly) */
    pitchDeg: Ref<number>

    /** Enable look-at tracking */
    enable: (target?: VrmLookAtTarget) => void

    /** Disable look-at tracking and reset to neutral */
    disable: () => void

    /** Update the tracking target */
    setTarget: (target: VrmLookAtTarget) => void

    /** Update controller config at runtime */
    setConfig: (patch: Partial<LookAtConfig>) => void

    /** Pause the update loop (keeps state) */
    pause: () => void

    /** Resume the update loop */
    resume: () => void

    /** Completely destroy and clean up */
    destroy: () => void
}

// ── Radian-to-degree conversion ──────────────────────────────────

function radToDeg(rad: number): number {
    return (rad * 180) / Math.PI
}

// ── Composable ───────────────────────────────────────────────────

export function useLookAt(options: UseLookAtOptions = {}): UseLookAtResult {
    // ── Reactive state ──────────────────────────────────────────

    const controller = shallowRef<LookAtController | null>(null)
    const state = ref<LookAtState>({
        yaw: 0,
        pitch: 0,
        tracking: false,
        target: { x: 0, y: 1.5, z: 4 },
    })
    const isEnabled = ref(false)
    const isTracking = ref(false)
    const yawDeg = ref(0)
    const pitchDeg = ref(0)

    // ── Internal ─────────────────────────────────────────────────

    const initialTarget: VrmLookAtTarget =
        (options.target && typeof options.target === 'object' && 'value' in options.target)
            ? (options.target as Ref<VrmLookAtTarget>).value
            : (options.target as VrmLookAtTarget) ?? { x: 0, y: 1.5, z: 4 }

    let updateIntervalMs = 0
    if (options.updateRate && options.updateRate > 0) {
        updateIntervalMs = 1000 / options.updateRate
    }

    let animationFrameId: number | null = null
    let lastUpdateTime = 0
    let disposed = false

    // Initialize controller
    controller.value = new LookAtController(options.config)
    controller.value.setTarget(initialTarget)

    // ── Update loop ──────────────────────────────────────────────

    function updateLoop(timestamp: number): void {
        if (disposed || !controller.value) return

        animationFrameId = requestAnimationFrame(updateLoop)

        if (updateIntervalMs > 0) {
            const elapsed = timestamp - lastUpdateTime
            if (elapsed < updateIntervalMs) return
            lastUpdateTime = timestamp
        }

        const deltaTime = lastUpdateTime === 0
            ? 0.016  // Assume ~60fps on first frame
            : (timestamp - lastUpdateTime) / 1000
        lastUpdateTime = timestamp

        const newState = controller.value.update(deltaTime)
        state.value = newState
        isTracking.value = newState.tracking
        yawDeg.value = radToDeg(newState.yaw)
        pitchDeg.value = radToDeg(newState.pitch)
    }

    // ── Public API ───────────────────────────────────────────────

    function startLoop(): void {
        if (disposed || animationFrameId !== null) return
        lastUpdateTime = 0
        animationFrameId = requestAnimationFrame(updateLoop)

        addVoiceLogEvent({
            level: 'info',
            source: 'vrm-animation',
            message: 'Look-at update loop started',
        })
    }

    function stopLoop(): void {
        if (animationFrameId !== null) {
            cancelAnimationFrame(animationFrameId)
            animationFrameId = null
        }
        lastUpdateTime = 0
    }

    function enable(target?: VrmLookAtTarget): void {
        if (!controller.value) return
        controller.value.enable(target)
        isEnabled.value = true

        // Resume tracking with optional new target
        if (target) {
            controller.value.setTarget(target)
        }

        startLoop()

        addVoiceLogEvent({
            level: 'info',
            source: 'vrm-animation',
            message: target
                ? `Look-at enabled, target: (${target.x.toFixed(1)}, ${target.y.toFixed(1)}, ${target.z.toFixed(1)})`
                : 'Look-at enabled',
        })
    }

    function disable(): void {
        if (!controller.value) return
        controller.value.disable()
        isEnabled.value = false
        isTracking.value = false
        yawDeg.value = 0
        pitchDeg.value = 0

        stopLoop()

        // Reset state to neutral
        state.value = {
            yaw: 0,
            pitch: 0,
            tracking: false,
            target: { x: 0, y: 1.5, z: 4 },
        }

        addVoiceLogEvent({
            level: 'info',
            source: 'vrm-animation',
            message: 'Look-at disabled',
        })
    }

    function setTarget(target: VrmLookAtTarget): void {
        controller.value?.setTarget(target)
    }

    function setConfig(patch: Partial<LookAtConfig>): void {
        controller.value?.setConfig(patch)
    }

    function pause(): void {
        stopLoop()
    }

    function resume(): void {
        if (isEnabled.value) {
            startLoop()
        }
    }

    // ── Reactivity: watch enabled/target options ─────────────────

    if (options.enabled !== undefined) {
        const enabledRef = typeof options.enabled === 'boolean'
            ? ref(options.enabled)
            : options.enabled

        watch(enabledRef, (val) => {
            if (val) {
                enable()
            } else {
                disable()
            }
        }, { immediate: true })
    }

    if (options.target && typeof options.target === 'object' && 'value' in options.target) {
        watch(options.target as Ref<VrmLookAtTarget>, (newTarget) => {
            if (newTarget) {
                setTarget(newTarget)
            }
        }, { deep: true })
    }

    // ── Cleanup ──────────────────────────────────────────────────

    function destroy(): void {
        disposed = true
        stopLoop()
        controller.value?.reset()
        controller.value = null
        isEnabled.value = false
        isTracking.value = false
    }

    onUnmounted(() => {
        destroy()
    })

    // ── Return ───────────────────────────────────────────────────

    return {
        controller,
        state,
        isEnabled,
        isTracking,
        yawDeg,
        pitchDeg,
        enable,
        disable,
        setTarget,
        setConfig,
        pause,
        resume,
        destroy,
    }
}
