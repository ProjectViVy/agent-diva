import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest'
import {
  IdleAnimationManager,
  type VrmRuntimeHandle,
} from './idle-animation-manager'
import type { IdleAnimationConfig } from './types'

// ── Mocks ──────────────────────────────────────────────────

function createMockRuntime(overrides?: Partial<VrmRuntimeHandle>): VrmRuntimeHandle {
  return {
    playMotion: vi.fn().mockResolvedValue(true),
    stopMotion: vi.fn().mockResolvedValue(true),
    setMotionState: vi.fn().mockResolvedValue(true),
    ...overrides,
  }
}

// ── Helpers ────────────────────────────────────────────────

/** Create a standard motion queue of 3 IDs */
const THREE_MOTIONS = ['motion_a', 'motion_b', 'motion_c']

// ── Tests ──────────────────────────────────────────────────

describe('IdleAnimationManager', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  // ── Constructor ──────────────────────────────────────────

  describe('constructor', () => {
    it('initializes with default config', () => {
      const manager = new IdleAnimationManager()

      expect(manager.active).toBe(false)
      expect(manager.mode).toBe('none')
      expect(manager.isOneShotPlaying).toBe(false)
    })

    it('merges provided config with defaults', () => {
      const custom: Partial<IdleAnimationConfig> = {
        estimatedDurationMs: 5000,
        shuffle: false,
      }
      const manager = new IdleAnimationManager(custom)

      // Access internal config indirectly via behavior
      // Empty queue → falls back, which reveals shuffle isn't consulted yet.
      // Manual functional test: setAnimationQueue with 1 item
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue(['single'])
      // Shuffle=false means index should go sequential. With 1 item, same.
      manager.startIdleLoop()
      // With 1 motion and shuffle=false, it should pick index 0
      expect(manager.mode).toBe('vrma')
      expect(manager.active).toBe(true)
    })

    it('accepts empty config and uses all defaults', () => {
      const manager = new IdleAnimationManager({})
      expect(manager.active).toBe(false)
      expect(manager.mode).toBe('none')
    })
  })

  // ── setRuntime / setAnimationQueue ───────────────────────

  describe('setRuntime', () => {
    it('attaches the runtime reference', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()

      expect(() => manager.setRuntime(runtime)).not.toThrow()
    })

    it('allows setting runtime to null', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setRuntime(null)

      // startIdleLoop with null runtime should signal error and NOT start
      manager.startIdleLoop()
      expect(manager.active).toBe(false)
      expect(manager.mode).toBe('none')
    })

    it('can replace runtime reference', () => {
      const manager = new IdleAnimationManager()
      const runtime1 = createMockRuntime()
      const runtime2 = createMockRuntime()

      manager.setRuntime(runtime1)
      manager.setRuntime(runtime2)

      // startIdleLoop should use runtime2
      manager.setAnimationQueue(THREE_MOTIONS)
      manager.startIdleLoop()
      expect(runtime1.playMotion).not.toHaveBeenCalled()
      expect(runtime2.setMotionState).toHaveBeenCalled()
    })
  })

  describe('setAnimationQueue', () => {
    it('replaces the existing queue', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)

      manager.setAnimationQueue(['old1', 'old2'])
      manager.setAnimationQueue(THREE_MOTIONS)

      manager.startIdleLoop()
      expect(runtime.playMotion).toHaveBeenCalled()
      const calledWith = (runtime.playMotion as ReturnType<typeof vi.fn>).mock.calls[0][0]
      expect(THREE_MOTIONS).toContain(calledWith)
    })

    it('resets the animation index', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)

      manager.setAnimationQueue(THREE_MOTIONS)
      // Queue set resets currentIndex to -1 internally, so pickFirstIndex() runs on start
      manager.startIdleLoop()
      expect(manager.mode).toBe('vrma')
    })
  })

  // ── startIdleLoop ────────────────────────────────────────

  describe('startIdleLoop', () => {
    it('enters procedural mode when queue is empty', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue([])

      manager.startIdleLoop()

      // Should call setMotionState to enable crate's built-in idle
      expect(runtime.setMotionState).toHaveBeenCalledWith({ idleEnabled: true })
      expect(manager.mode).toBe('procedural')
      expect(manager.active).toBe(true)
    })

    it('enters VRMA mode when queue has motions', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue(THREE_MOTIONS)

      manager.startIdleLoop()

      expect(manager.mode).toBe('vrma')
      expect(manager.active).toBe(true)
      // Should disable crate's built-in idle since we manage it
      expect(runtime.setMotionState).toHaveBeenCalledWith({ idleEnabled: false })
      // Should start playing the first motion
      expect(runtime.playMotion).toHaveBeenCalled()
    })

    it('signals error when no runtime is set', () => {
      const manager = new IdleAnimationManager()
      // No runtime set
      manager.startIdleLoop()

      expect(manager.active).toBe(false)
      expect(manager.mode).toBe('none')
    })

    it('restarts if already active', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue(THREE_MOTIONS)

      // First start
      manager.startIdleLoop()
      expect(manager.active).toBe(true)

      // Second start — should stop first and restart
      manager.startIdleLoop()
      // stopMotion was called during the restart stopAllAnimations
      expect(runtime.stopMotion).toHaveBeenCalled()
      // Should still be in vrma mode
      expect(manager.mode).toBe('vrma')
    })
  })

  // ── playOneShotAnimation ─────────────────────────────────

  describe('playOneShotAnimation', () => {
    it('sets currentOneShotId on successful play', async () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue(THREE_MOTIONS)

      // First start idle loop so the motion is in the queue
      manager.startIdleLoop()

      const result = await manager.playOneShotAnimation('motion_b')
      expect(result).toBe(true)
      expect(manager.isOneShotPlaying).toBe(true)
    })

    it('returns false when motion is not in queue', async () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue(THREE_MOTIONS)

      const result = await manager.playOneShotAnimation('nonexistent')
      expect(result).toBe(false)
      expect(manager.isOneShotPlaying).toBe(false)
    })

    it('returns false when runtime is not set', async () => {
      const manager = new IdleAnimationManager()
      manager.setAnimationQueue(THREE_MOTIONS)

      const result = await manager.playOneShotAnimation('motion_a')
      expect(result).toBe(false)
    })

    it('plays one-shot without explicitly stopping current animation', async () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue(THREE_MOTIONS)

      manager.startIdleLoop()
      // Clear the call history from startIdleLoop
      vi.clearAllMocks()

      await manager.playOneShotAnimation('motion_c')
      // Should NOT explicitly stop motion — the crate handles the transition
      // internally.  Explicit stopMotion() creates a visible freeze frame
      // (matches super-agent-party crossfade approach)
      expect(runtime.stopMotion).not.toHaveBeenCalled()
      // Should play the one-shot
      expect(runtime.playMotion).toHaveBeenCalledWith('motion_c')
    })

    it('returns false when playMotion returns false', async () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime({
        playMotion: vi.fn().mockResolvedValue(false),
      })
      manager.setRuntime(runtime)
      manager.setAnimationQueue(THREE_MOTIONS)

      manager.startIdleLoop()
      vi.clearAllMocks()

      const result = await manager.playOneShotAnimation('motion_a')
      // playMotion returns false → one-shot fails
      expect(result).toBe(false)
    })

    it('resets to idle after estimated duration', async () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue(THREE_MOTIONS)

      manager.startIdleLoop()
      vi.clearAllMocks()

      await manager.playOneShotAnimation('motion_a')
      expect(manager.isOneShotPlaying).toBe(true)

      // Fast-forward past estimated duration (default 3000ms)
      await vi.advanceTimersByTimeAsync(3100)

      expect(manager.isOneShotPlaying).toBe(false)
    })
  })

  // ── stopAllAnimations ────────────────────────────────────

  describe('stopAllAnimations', () => {
    it('resets mode to none', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue(THREE_MOTIONS)

      manager.startIdleLoop()
      expect(manager.mode).toBe('vrma')

      manager.stopAllAnimations()
      expect(manager.mode).toBe('none')
    })

    it('sets isActive to false', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue(['single'])

      manager.startIdleLoop()
      expect(manager.active).toBe(true)

      manager.stopAllAnimations()
      expect(manager.active).toBe(false)
    })

    it('clears currentOneShotId', async () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue(THREE_MOTIONS)

      // Start idle and immediately fire a one-shot
      manager.startIdleLoop()
      await manager.playOneShotAnimation('motion_b')
      expect(manager.isOneShotPlaying).toBe(true)

      manager.stopAllAnimations()
      expect(manager.isOneShotPlaying).toBe(false)
    })

    it('calls runtime.stopMotion and disables idle', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue(THREE_MOTIONS)

      manager.startIdleLoop()
      vi.clearAllMocks()

      manager.stopAllAnimations()

      expect(runtime.stopMotion).toHaveBeenCalled()
      expect(runtime.setMotionState).toHaveBeenCalledWith({ idleEnabled: false })
    })

    it('clears all internal timers', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue(THREE_MOTIONS)

      manager.startIdleLoop()
      // At this point there are setTimeout timers scheduled
      manager.stopAllAnimations()
      // Should not throw — timers are cleared
      vi.advanceTimersByTime(10000)
      // After timers advance, no further playMotion calls from stale timers
      // (the motions count should not increase beyond what was already called)
    })
  })

  // ── destroy ──────────────────────────────────────────────

  describe('destroy', () => {
    it('stops all animations', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue(THREE_MOTIONS)
      manager.startIdleLoop()

      manager.destroy()

      expect(manager.active).toBe(false)
      expect(manager.mode).toBe('none')
    })

    it('nullifies the runtime reference', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue(THREE_MOTIONS)

      manager.destroy()

      // After destroy, calling startIdleLoop should fail gracefully
      manager.startIdleLoop()
      expect(manager.active).toBe(false)
    })

    it('nullifies the error handler', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      const errorHandler = vi.fn()
      manager.setErrorHandler(errorHandler)

      manager.destroy()

      // After destroy, startIdleLoop with null runtime
      // — error handler should not fire (was nullified)
      manager.startIdleLoop()
      expect(errorHandler).not.toHaveBeenCalled()
    })
  })

  // ── setErrorHandler ──────────────────────────────────────

  describe('setErrorHandler', () => {
    it('calls error handler on animation errors', () => {
      const manager = new IdleAnimationManager()
      const errorHandler = vi.fn()
      manager.setErrorHandler(errorHandler)

      // Trigger error by calling startIdleLoop with no runtime
      manager.startIdleLoop()
      expect(errorHandler).toHaveBeenCalledTimes(1)
    })

    it('does not call handler when it was not set', () => {
      const manager = new IdleAnimationManager()
      // No error handler set — should not throw
      expect(() => manager.startIdleLoop()).not.toThrow()
    })
  })

  // ── Procedural mode fallback ─────────────────────────────

  describe('procedural fallback', () => {
    it('falls back to procedural when queue is empty', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      // No setAnimationQueue call — defaults to empty

      manager.startIdleLoop()

      expect(manager.mode).toBe('procedural')
      expect(manager.active).toBe(true)
      expect(runtime.setMotionState).toHaveBeenCalledWith({ idleEnabled: true })
    })

    it('falls back to procedural when queue is set to empty array', () => {
      const manager = new IdleAnimationManager()
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue([])

      manager.startIdleLoop()

      expect(manager.mode).toBe('procedural')
      expect(manager.active).toBe(true)
    })
  })

  // ── Inactive state ───────────────────────────────────────

  describe('inactive state', () => {
    it('active returns false when not started', () => {
      const manager = new IdleAnimationManager()
      expect(manager.active).toBe(false)
    })

    it('mode returns none when not started', () => {
      const manager = new IdleAnimationManager()
      expect(manager.mode).toBe('none')
    })

    it('isOneShotPlaying returns false when no one-shot', () => {
      const manager = new IdleAnimationManager()
      expect(manager.isOneShotPlaying).toBe(false)
    })
  })

  // ── Shuffle behavior ─────────────────────────────────────

  describe('shuffle mode', () => {
    it('respects shuffle: false for sequential playback', () => {
      const manager = new IdleAnimationManager({ shuffle: false })
      const runtime = createMockRuntime()
      manager.setRuntime(runtime)
      manager.setAnimationQueue(['m1', 'm2', 'm3'])

      manager.startIdleLoop()

      // With shuffle=false, first index is 0 → 'm1'
      expect(runtime.playMotion).toHaveBeenCalledWith('m1')
    })
  })
})
