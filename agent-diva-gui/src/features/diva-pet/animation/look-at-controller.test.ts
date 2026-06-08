import { describe, expect, it } from 'vitest'
import {
  LookAtController,
  DEFAULT_LOOK_AT_CONFIG,
  type LookAtConfig,
  type LookAtState,
} from './look-at-controller'
import type { VrmLookAtTarget } from '../types'

// ── Helpers ────────────────────────────────────────────────

/** Create a target at a given world-space position */
function makeTarget(x: number, y: number, z: number): VrmLookAtTarget {
  return { x, y, z }
}

// ── Tests ──────────────────────────────────────────────────

describe('LookAtController', () => {
  // ── Constructor ──────────────────────────────────────────

  describe('constructor', () => {
    it('uses DEFAULT_LOOK_AT_CONFIG by default', () => {
      const controller = new LookAtController()
      const config = controller.getConfig()

      expect(config.yawLimitDeg).toBe(DEFAULT_LOOK_AT_CONFIG.yawLimitDeg)
      expect(config.pitchUpLimitDeg).toBe(DEFAULT_LOOK_AT_CONFIG.pitchUpLimitDeg)
      expect(config.pitchDownLimitDeg).toBe(DEFAULT_LOOK_AT_CONFIG.pitchDownLimitDeg)
      expect(config.behindLimitDeg).toBe(DEFAULT_LOOK_AT_CONFIG.behindLimitDeg)
      expect(config.lerpSpeed).toBe(DEFAULT_LOOK_AT_CONFIG.lerpSpeed)
      expect(config.neckRatio).toBe(DEFAULT_LOOK_AT_CONFIG.neckRatio)
      expect(config.headRatio).toBe(DEFAULT_LOOK_AT_CONFIG.headRatio)
    })

    it('merges partial config with defaults', () => {
      const partial: Partial<LookAtConfig> = {
        yawLimitDeg: 30,
        lerpSpeed: 5.0,
      }
      const controller = new LookAtController(partial)
      const config = controller.getConfig()

      expect(config.yawLimitDeg).toBe(30)
      expect(config.lerpSpeed).toBe(5.0)
      // Unspecified fields use defaults
      expect(config.pitchUpLimitDeg).toBe(DEFAULT_LOOK_AT_CONFIG.pitchUpLimitDeg)
    })

    it('is not enabled initially', () => {
      const controller = new LookAtController()
      expect(controller.isEnabled).toBe(false)
    })

    it('is not tracking initially', () => {
      const controller = new LookAtController()
      expect(controller.tracking).toBe(false)
    })
  })

  // ── enable / disable ─────────────────────────────────────

  describe('enable', () => {
    it('sets enabled to true', () => {
      const controller = new LookAtController()
      controller.enable()
      expect(controller.isEnabled).toBe(true)
    })

    it('resets interpolation state on enable', () => {
      const controller = new LookAtController()

      // First enable, set a target, and update to get some yaw/pitch
      controller.enable(makeTarget(2, 1.5, 4))
      // Update multiple frames to accumulate some interpolation
      for (let i = 0; i < 10; i++) {
        controller.update(0.016)
      }

      // Now re-enable — interpolation state should reset
      controller.enable()
      const state = controller.update(0.016)
      // Should be near zero since we just reset
      expect(Math.abs(state.yaw)).toBeLessThan(1.0)
      expect(Math.abs(state.pitch)).toBeLessThan(1.0)
    })

    it('can set a target during enable', () => {
      const controller = new LookAtController()
      const target: VrmLookAtTarget = { x: 3, y: 1.2, z: 5 }
      controller.enable(target)

      expect(controller.isEnabled).toBe(true)
    })
  })

  describe('disable', () => {
    it('sets enabled to false', () => {
      const controller = new LookAtController()
      controller.enable()
      expect(controller.isEnabled).toBe(true)

      controller.disable()
      expect(controller.isEnabled).toBe(false)
    })

    it('resets tracking state and angles', () => {
      const controller = new LookAtController()
      controller.enable(makeTarget(3, 1.5, 4))

      // Do a few updates to build up angles
      for (let i = 0; i < 5; i++) {
        controller.update(0.016)
      }

      controller.disable()
      expect(controller.tracking).toBe(false)

      // Next update should return near-zero angles
      const state = controller.update(0.016)
      expect(Math.abs(state.yaw)).toBeLessThan(0.01)
      expect(Math.abs(state.pitch)).toBeLessThan(0.01)
    })
  })

  // ── update ───────────────────────────────────────────────

  describe('update', () => {
    it('returns zero angles when disabled', () => {
      const controller = new LookAtController()
      // Not enabled

      const state = controller.update(0.016)
      expect(state.yaw).toBe(0)
      expect(state.pitch).toBe(0)
      expect(state.tracking).toBe(false)
    })

    it('computes yaw when target is to the right', () => {
      const controller = new LookAtController()
      // Target to the right (positive X)
      controller.enable(makeTarget(3, 1.5, 4))

      // Multiple updates to lerp toward target
      let state!: LookAtState
      for (let i = 0; i < 30; i++) {
        state = controller.update(0.016)
      }

      // Yaw should be positive (looking right)
      expect(state.yaw).toBeGreaterThan(0)
      expect(state.pitch).toBeCloseTo(0, 1) // at same height
    })

    it('computes yaw when target is to the left', () => {
      const controller = new LookAtController()
      controller.enable(makeTarget(-3, 1.5, 4))

      let state!: LookAtState
      for (let i = 0; i < 30; i++) {
        state = controller.update(0.016)
      }

      // Yaw should be negative (looking left)
      expect(state.yaw).toBeLessThan(0)
    })

    it('computes pitch when target is above', () => {
      const controller = new LookAtController()
      controller.enable(makeTarget(0, 4, 4))

      let state!: LookAtState
      for (let i = 0; i < 30; i++) {
        state = controller.update(0.016)
      }

      // Pitch should be positive (looking up)
      expect(state.pitch).toBeGreaterThan(0)
    })

    it('computes pitch when target is below', () => {
      const controller = new LookAtController()
      controller.enable(makeTarget(0, 0, 4))

      let state!: LookAtState
      for (let i = 0; i < 30; i++) {
        state = controller.update(0.016)
      }

      // Pitch should be negative (looking down)
      expect(state.pitch).toBeLessThan(0)
    })

    it('returns tracking=true when looking at target in front', () => {
      const controller = new LookAtController()
      controller.enable(makeTarget(1, 1.5, 4))

      let state!: LookAtState
      for (let i = 0; i < 30; i++) {
        state = controller.update(0.016)
      }

      expect(state.tracking).toBe(true)
    })
  })

  // ── Behind target (> 110°) ───────────────────────────────

  describe('behind target', () => {
    it('does not track when target is behind (> 110°)', () => {
      const controller = new LookAtController()
      // Target behind model: negative Z with large enough angle
      // atan2(0, -4) = PI = 180°, scaled by 0.6 = 108°, > 110°? No.
      // Need to compute: default behindLimitDeg is 110°, so need rawYaw > 110°
      // rawYaw = atan2(dx, dz). For target behind right: atan2(5, -1) ≈ 1.768 rad ≈ 101°
      // Let's use a target directly behind: atan2(1, -0.3) ≈ 1.86 rad ≈ 106.7°, scaled 0.6 ≈ 64°
      // Hmm, the code checks Math.abs(rawYaw) > behindLimitRad (110° in radians = 1.9199)
      // So rawYaw must be > 110° absolute.
      // atan2(5, -0.5) = atan2(5, -0.5) ≈ 1.67 rad ≈ 95.7°. Not enough.
      // Target at: x=10, z=-0.1 → atan2(10, -0.1) ≈ 1.58 rad → 90.5°. Not enough.
      // The yaw can't exceed 180° (π). atan2(dx, dz) where dz is tiny negative.
      // atan2(1, -0.01) ≈ 1.56 rad ≈ 89.4°. So raw yaw is bounded to < π.
      // Actually atan2 returns (-π, π]. So atan2(1, -0.01) → π - 0.01 ≈ 3.13 rad ≈ 179.4°
      // Ah! For atan2(y, x), when x is negative:
      // atan2(1, -0.01) → arctan(1/-0.01) + π = arctan(-100) + π ≈ -1.56 + 3.14 = 1.57 rad
      // Wait, let me just think more carefully.
      // Target behind: z is negative, say x=4, z=-2
      // dx = 4-0 = 4, dz = -2-0 = -2
      // rawYaw = atan2(4, -2) = atan2(4, -2)
      // For atan2(positive, negative), result is in quadrant II: > π/2 and < π
      // atan2(4, -2) ≈ π - atan(4/2) = π - 1.107 = 2.034 rad ≈ 116.5°
      // That's > 110°, so should trigger behindLimit!
      // scaled by 0.6: 2.034 * 0.6 = 1.22, but rawYaw 2.034 > 1.9199 (110° in rad)
      // So targetYaw = 0, targetPitch = 0, isTracking = false

      // Let me just use a clearly behind target
      controller.enable(makeTarget(5, 1.5, -2))

      let state!: LookAtState
      for (let i = 0; i < 30; i++) {
        state = controller.update(0.016)
      }

      // Should not be tracking
      expect(state.tracking).toBe(false)
      // Should lerp toward zero
      expect(Math.abs(state.yaw)).toBeLessThan(0.1)
    })

    it('still tracks when target is within behind limit', () => {
      const controller = new LookAtController()
      // Target slightly to the side: atan2(4, 2) = 1.107 rad = 63.4° < 110°
      controller.enable(makeTarget(4, 1.5, 2))

      let state!: LookAtState
      for (let i = 0; i < 30; i++) {
        state = controller.update(0.016)
      }

      expect(state.tracking).toBe(true)
      expect(state.yaw).toBeGreaterThan(0)
    })
  })

  // ── Angle clamping ───────────────────────────────────────

  describe('angle clamping', () => {
    it('clamps yaw at ±45° limits', () => {
      const controller = new LookAtController()
      // Target far to the right: atan2(10, 0.5) ≈ 1.52 rad ≈ 87°
      // scaled by 0.6: 0.912 rad ≈ 52° → should clamp to 45° = 0.785 rad
      controller.enable(makeTarget(10, 1.5, 0.5))

      let state!: LookAtState
      for (let i = 0; i < 100; i++) {
        state = controller.update(0.016)
      }

      // Yaw should be clamped at or below 45° = 0.785 rad
      const yawLimitRad = (45 * Math.PI) / 180 // 0.785...
      expect(Math.abs(state.yaw)).toBeLessThanOrEqual(yawLimitRad + 0.001)
    })

    it('clamps pitch at +40° upper limit', () => {
      const controller = new LookAtController()
      // Target very high: y=10, horizontal=1
      // rawPitch = atan2(9, 1) ≈ 1.46 rad ≈ 83.6°
      // scaled by 0.6: 0.876 rad ≈ 50.2° → should clamp to 40° = 0.698 rad
      controller.enable(makeTarget(0, 10, 1))

      let state!: LookAtState
      for (let i = 0; i < 100; i++) {
        state = controller.update(0.016)
      }

      const pitchUpLimitRad = (40 * Math.PI) / 180 // 0.698...
      expect(state.pitch).toBeLessThanOrEqual(pitchUpLimitRad + 0.001)
    })

    it('clamps pitch at -20° lower limit', () => {
      const controller = new LookAtController()
      // Target very low: y=-3, horizontal=1
      // rawPitch = atan2(-4.5, 1) ≈ -1.352 rad ≈ -77.5°
      // scaled by 0.6: -0.811 rad ≈ -46.5° → clamped to -20° = -0.349 rad
      controller.enable(makeTarget(0, -3, 1))

      let state!: LookAtState
      for (let i = 0; i < 100; i++) {
        state = controller.update(0.016)
      }

      const pitchDownLimitRad = (20 * Math.PI) / 180 // 0.349...
      expect(state.pitch).toBeGreaterThanOrEqual(-pitchDownLimitRad - 0.001)
    })

    it('respects custom yaw limit', () => {
      const controller = new LookAtController({ yawLimitDeg: 20 })
      controller.enable(makeTarget(10, 1.5, 0.5))

      let state!: LookAtState
      for (let i = 0; i < 100; i++) {
        state = controller.update(0.016)
      }

      const customYawLimitRad = (20 * Math.PI) / 180
      expect(Math.abs(state.yaw)).toBeLessThanOrEqual(customYawLimitRad + 0.001)
    })
  })

  // ── setTarget ────────────────────────────────────────────

  describe('setTarget', () => {
    it('updates the target position', () => {
      const controller = new LookAtController()
      controller.enable(makeTarget(1, 1.5, 4))

      // Now change target
      controller.setTarget(makeTarget(-2, 1.5, 3))

      // Multiple updates to react to new target
      let state!: LookAtState
      for (let i = 0; i < 30; i++) {
        state = controller.update(0.016)
      }

      // Should now be looking left
      expect(state.yaw).toBeLessThan(0)
    })

    it('clones the target object', () => {
      const controller = new LookAtController()
      const mutableTarget: VrmLookAtTarget = { x: 5, y: 1.5, z: 4 }
      controller.setTarget(mutableTarget)

      // Mutating original should not affect controller
      mutableTarget.x = 100

      // The debug info should reflect what was set, not the mutated value
      const debug = controller.getDebugInfo()
      expect(debug.target.x).toBe(5)
    })
  })

  // ── reset ────────────────────────────────────────────────

  describe('reset', () => {
    it('zeros yaw and pitch internally', () => {
      const controller = new LookAtController()
      controller.enable(makeTarget(5, 1.5, 4))

      // Build up some angles
      for (let i = 0; i < 30; i++) {
        controller.update(0.016)
      }

      controller.reset()

      // After reset, internal angles should be zero via getDebugInfo
      const debug = controller.getDebugInfo()
      // reset() zeroes currentYaw and currentPitch. Since it does NOT
      // disable the controller, calling update() would re-compute angles.
      // So we check the internal state directly through debug info.
      expect(debug.currentYawDeg).toBe(0)
      expect(debug.currentPitchDeg).toBe(0)
    })

    it('zeros internal tracking state', () => {
      const controller = new LookAtController()
      controller.enable(makeTarget(3, 1.5, 4))

      for (let i = 0; i < 30; i++) {
        controller.update(0.016)
      }

      // Verify tracking is active before reset
      const stateBefore = controller.update(0.016)
      expect(stateBefore.tracking).toBe(true)

      controller.reset()

      // After reset, internal isTracking is false (but controller is still enabled,
      // so the next update() will re-set isTracking based on the target position).
      // Verify via getDebugInfo()
      const debug = controller.getDebugInfo()
      expect(debug.tracking).toBe(false)
    })

    it('does NOT disable the controller', () => {
      const controller = new LookAtController()
      controller.enable()
      expect(controller.isEnabled).toBe(true)

      controller.reset()
      // reset() should NOT change enabled state
      expect(controller.isEnabled).toBe(true)
    })
  })

  // ── getBoneRotations ─────────────────────────────────────

  describe('getBoneRotations', () => {
    it('returns neck and head rotation pairs', () => {
      const controller = new LookAtController()
      controller.enable(makeTarget(2, 1.5, 4))

      // Build up some angles
      for (let i = 0; i < 30; i++) {
        controller.update(0.016)
      }

      const rotations = controller.getBoneRotations()
      expect(rotations).toHaveProperty('neck')
      expect(rotations).toHaveProperty('head')
      expect(rotations.neck).toHaveProperty('yaw')
      expect(rotations.neck).toHaveProperty('pitch')
      expect(rotations.head).toHaveProperty('yaw')
      expect(rotations.head).toHaveProperty('pitch')

      // Neck gets full ratio (1.0 by default)
      expect(rotations.neck.yaw).not.toBe(0)
      // Head gets additive fraction (0.5 by default)
      expect(Math.abs(rotations.head.yaw)).toBeLessThanOrEqual(Math.abs(rotations.neck.yaw))
    })
  })

  // ── getDebugInfo ─────────────────────────────────────────

  describe('getDebugInfo', () => {
    it('returns current state in degrees', () => {
      const controller = new LookAtController()
      controller.enable(makeTarget(2, 1.5, 4))

      for (let i = 0; i < 30; i++) {
        controller.update(0.016)
      }

      const debug = controller.getDebugInfo()
      expect(debug).toHaveProperty('currentYawDeg')
      expect(debug).toHaveProperty('currentPitchDeg')
      expect(debug).toHaveProperty('target')
      expect(debug).toHaveProperty('tracking')
      expect(debug).toHaveProperty('enabled')
      expect(debug.enabled).toBe(true)
    })

    it('clones the target to avoid mutation', () => {
      const controller = new LookAtController()
      controller.setTarget(makeTarget(3, 1.5, 4))

      const debug1 = controller.getDebugInfo()
      debug1.target.x = 999

      const debug2 = controller.getDebugInfo()
      expect(debug2.target.x).toBe(3)
    })
  })

  // ── setModelOrigin ───────────────────────────────────────

  describe('setModelOrigin', () => {
    it('changes the reference origin for look-at calculation', () => {
      const controller = new LookAtController()
      controller.setModelOrigin({ x: 1, y: 2, z: 0 })
      controller.enable(makeTarget(1, 2, 4))

      // With origin at (1,2,0) and target at (1,2,4), dx=0, dy=0, dz=4
      // Should be looking straight ahead with no yaw/pitch
      let state!: LookAtState
      for (let i = 0; i < 30; i++) {
        state = controller.update(0.016)
      }

      expect(state.yaw).toBeCloseTo(0, 2)
      expect(state.pitch).toBeCloseTo(0, 2)
    })
  })

  // ── setConfig ────────────────────────────────────────────

  describe('setConfig', () => {
    it('updates configuration at runtime', () => {
      const controller = new LookAtController()
      controller.setConfig({ lerpSpeed: 10.0 })

      expect(controller.getConfig().lerpSpeed).toBe(10.0)
    })

    it('partial update preserves other configs', () => {
      const controller = new LookAtController()
      controller.setConfig({ yawLimitDeg: 60 })

      const config = controller.getConfig()
      expect(config.yawLimitDeg).toBe(60)
      expect(config.pitchUpLimitDeg).toBe(DEFAULT_LOOK_AT_CONFIG.pitchUpLimitDeg)
    })
  })

  // ── Edge case: directly above/below ──────────────────────

  describe('directly above/below target', () => {
    it('maintains current angles when target is directly above', () => {
      const controller = new LookAtController()
      // Target directly above: horizontal distance is 0
      controller.enable(makeTarget(0, 5, 0))

      const state = controller.update(0.016)
      // Should not track (horizontalDist < 0.001)
      expect(state.tracking).toBe(false)
      // Maintained current angles (0 initially)
      expect(state.yaw).toBe(0)
      expect(state.pitch).toBe(0)
    })
  })
})
