/**
 * Vue composable for VRMA animation management.
 *
 * Wraps IdleAnimationManager and ProceduralIdleGenerator into a Vue
 * reactivity-compatible API surface.  Handles VRMA file loading,
 * lifecycle management, and error logging.
 *
 * Pattern: Follows useVoicePlayer/useVoiceInput conventions
 *  - Options interface for input
 *  - Result interface for output
 *  - ref() for reactive state, shallowRef() for heavy objects
 *  - onUnmounted() cleanup
 *  - addVoiceLogEvent() for error logging
 */

import { type Ref, type ShallowRef, onUnmounted, ref, shallowRef } from 'vue'
import type { PetConfig, VrmMotionInfo } from '../types'
import { addVoiceLogEvent } from '../voice/services/voice-log'
import { IdleAnimationManager } from './idle-animation-manager'
import { ProceduralIdleGenerator } from './procedural-idle'
import type {
  AnimationMode,
  MotionAnimData,
  MotionLoadResult,
} from './types'
import { VrmAnimationError } from './types'

// ── Options ─────────────────────────────────────────────────────

export interface UseVrmAnimationOptions {
  /** Pet config (reactive) for motion list reference */
  config: Ref<PetConfig>
}

// ── Result ──────────────────────────────────────────────────────

export interface UseVrmAnimationResult {
  /** The IdleAnimationManager instance */
  idleManager: ShallowRef<IdleAnimationManager | null>

  /** The ProceduralIdleGenerator instance */
  proceduralIdle: ShallowRef<ProceduralIdleGenerator | null>

  /** Whether the idle animation system is currently active */
  isAnimating: Ref<boolean>

  /** Current animation mode */
  mode: Ref<AnimationMode>

  /** Current motion pool size */
  motionCount: Ref<number>

  /** Load VRMA animations from motion info list (fetches files) */
  loadAnimations: (motionList: VrmMotionInfo[]) => Promise<MotionAnimData[]>

  /** Play a one-shot motion by ID */
  playMotion: (id: string) => Promise<boolean>

  /** Play a one-shot motion by display name (for AI calling) */
  playMotionByName: (name: string) => Promise<boolean>

  /** Stop any currently playing one-shot motion */
  stopMotion: () => Promise<boolean>

  /** Start or restart the idle animation loop */
  startIdleLoop: () => Promise<void>

  /** Enable or disable all animations */
  setEnabled: (enabled: boolean) => void

  /** Completely destroy all animation resources */
  destroy: () => void
}

// ── Default estimated VRMA animation duration ───────────────────
const DEFAULT_MOTION_DURATION_MS = 3000

// ── Composable ──────────────────────────────────────────────────

export function useVrmAnimation(options: UseVrmAnimationOptions): UseVrmAnimationResult {
  const { config } = options

  // ── Reactive state ──────────────────────────────────────────

  const idleManager = shallowRef<IdleAnimationManager | null>(null)
  const proceduralIdle = shallowRef<ProceduralIdleGenerator | null>(null)
  const isAnimating = ref(false)
  const mode = ref<AnimationMode>('none')
  const motionCount = ref(0)

  /** Loaded motion data cache (id → MotionAnimData) */
  const motionCache = shallowRef<Map<string, MotionAnimData>>(new Map())

  // ── Initialize managers ─────────────────────────────────────

  function ensureManagers(): void {
    if (!idleManager.value) {
      idleManager.value = new IdleAnimationManager()

      // Wire up error logging via voice-log
      idleManager.value.setErrorHandler((error: VrmAnimationError, context: string) => {
        addVoiceLogEvent({
          level: 'warn',
          source: 'vrm-animation',
          message: `${context}: ${error.message}`,
          detail: { code: error.code },
        })
      })
    }

    if (!proceduralIdle.value) {
      proceduralIdle.value = new ProceduralIdleGenerator()
    }
  }

  // ── VRMA file loading ───────────────────────────────────────

  /**
   * Load VRMA animation binary files from the motion info list.
   *
   * Fetches each .vrma file and caches the parsed data.
   * Failed loads are caught individually so one bad file doesn't
   * prevent the others from loading.
   *
   * Strategy:
   *  1. Concurrently fetch all VRMA files via HEAD check then GET
   *  2. Cache ArrayBuffers in motionData map
   *  3. Report per-file errors via voice-log
   */
  async function loadAnimations(
    motionList: VrmMotionInfo[],
  ): Promise<MotionAnimData[]> {
    if (motionList.length === 0) {
      motionCount.value = 0
      return []
    }

    console.log(
      `[useVrmAnimation] Loading ${motionList.length} VRMA animations`,
    )

    const results: MotionLoadResult[] = await Promise.allSettled(
      motionList.map(async (motion): Promise<MotionLoadResult> => {
        try {
          // Check file exists first (avoids long timeouts on 404)
          const headResponse = await fetch(motion.path, { method: 'HEAD' })
          if (!headResponse.ok) {
            return {
              motion: null,
              success: false,
              error: `VRMA file not found: ${motion.path}`,
            }
          }

          const response = await fetch(motion.path)
          if (!response.ok) {
            return {
              motion: null,
              success: false,
              error: `Failed to fetch VRMA: ${motion.path} (${response.status})`,
            }
          }

          const arrayBuffer = await response.arrayBuffer()
          const motionData: MotionAnimData = {
            id: motion.id,
            name: motion.name,
            data: arrayBuffer,
            duration: DEFAULT_MOTION_DURATION_MS,
          }

          // Cache the loaded data
          const cache = motionCache.value
          cache.set(motion.id, motionData)

          return { motion: motionData, success: true }
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error)
          addVoiceLogEvent({
            level: 'warn',
            source: 'vrm-animation',
            message: `Failed to load VRMA: ${motion.name}`,
            detail: { path: motion.path, error: message },
          })
          return { motion: null, success: false, error: message }
        }
      }),
    ).then((settled) =>
      settled.map((r) =>
        r.status === 'fulfilled'
          ? r.value
          : { motion: null, success: false, error: String(r.reason) },
      ),
    )

    const loaded = results
      .filter((r): r is { motion: MotionAnimData; success: true } =>
        r.success && r.motion !== null,
      )
      .map((r) => r.motion)

    const failed = results.filter((r) => !r.success).length
    motionCount.value = loaded.length

    console.log(
      `[useVrmAnimation] Loaded ${loaded.length}/${motionList.length} animations` +
        (failed > 0 ? ` (${failed} failed)` : ''),
    )

    return loaded
  }

  // ── Animation control ───────────────────────────────────────

  /**
   * Start the idle animation loop with the current motion pool.
   *
   * Requires the VRM runtime to be set on the manager before calling.
   */
  async function startIdleLoop(): Promise<void> {
    ensureManagers()

    if (!idleManager.value) return

    // Set the motion pool from config
    const selectedIds = config.value.selectedMotionIds
    const allMotions = config.value.vrmMotionList

    if (selectedIds.length > 0) {
      // Only selected motions
      const validIds = selectedIds.filter((id) =>
        allMotions.some((m) => m.id === id),
      )
      idleManager.value.setAnimationQueue(validIds)
      motionCount.value = validIds.length
    } else if (allMotions.length > 0) {
      // All available motions
      idleManager.value.setAnimationQueue(allMotions.map((m) => m.id))
      motionCount.value = allMotions.length
    }

    await idleManager.value.startIdleLoop()
    isAnimating.value = true
    mode.value = 'vrma'

    addVoiceLogEvent({
      level: 'info',
      source: 'vrm-animation',
      message: 'Idle animation loop started',
      detail: { motionCount: motionCount.value },
    })
  }

  /** Play a one-shot motion — aliased to IdleAnimationManager */
  async function playMotion(id: string): Promise<boolean> {
    ensureManagers()
    if (!idleManager.value) return false
    return idleManager.value.playOneShotAnimation(id)
  }

  /** Play by display name (for AI semantic calling) */
  async function playMotionByName(name: string): Promise<boolean> {
    ensureManagers()
    if (!idleManager.value) return false

    // Resolve name to ID via config motion list
    const motion = config.value.vrmMotionList.find(
      (m) => m.name === name || m.id === name,
    )
    if (!motion) {
      addVoiceLogEvent({
        level: 'warn',
        source: 'vrm-animation',
        message: `Motion not found by name: "${name}"`,
      })
      return false
    }

    return idleManager.value.playOneShotAnimation(motion.id)
  }

  /** Stop any one-shot motion and restore idle */
  async function stopMotion(): Promise<boolean> {
    if (!idleManager.value) return false
    idleManager.value.stopAllAnimations()
    isAnimating.value = false
    mode.value = 'none'
    return true
  }

  /** Toggle the entire animation system on/off */
  function setEnabled(enabled: boolean): void {
    if (enabled) {
      void startIdleLoop()
    } else {
      idleManager.value?.stopAllAnimations()
      proceduralIdle.value?.stop()
      isAnimating.value = false
      mode.value = 'none'
    }
  }

  // ── Cleanup ─────────────────────────────────────────────────

  function destroy(): void {
    idleManager.value?.destroy()
    idleManager.value = null
    proceduralIdle.value?.destroy()
    proceduralIdle.value = null
    motionCache.value.clear()
    isAnimating.value = false
    mode.value = 'none'
    motionCount.value = 0
  }

  onUnmounted(() => {
    destroy()
  })

  // ── Return ──────────────────────────────────────────────────

  return {
    idleManager,
    proceduralIdle,
    isAnimating,
    mode,
    motionCount,
    loadAnimations,
    playMotion,
    playMotionByName,
    stopMotion,
    startIdleLoop,
    setEnabled,
    destroy,
  }
}

// ── Non-reactive helper ─────────────────────────────────────────

/**
 * Preload VRMA animation data without Vue reactivity overhead.
 * Useful for background loading before entering a composable context.
 */
export async function preloadVRMAAnimations(
  motionList: VrmMotionInfo[],
): Promise<MotionAnimData[]> {
  const results = await Promise.allSettled(
    motionList.map(async (motion): Promise<MotionAnimData | null> => {
      try {
        const response = await fetch(motion.path)
        if (!response.ok) return null
        const data = await response.arrayBuffer()
        return {
          id: motion.id,
          name: motion.name,
          data,
          duration: DEFAULT_MOTION_DURATION_MS,
        }
      } catch {
        return null
      }
    }),
  )

  return results
    .filter(
      (r): r is PromiseFulfilledResult<MotionAnimData> =>
        r.status === 'fulfilled' && r.value !== null,
    )
    .map((r) => r.value)
}
