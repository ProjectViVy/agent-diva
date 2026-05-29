import { describe, expect, it } from 'vitest'
import { getBuiltinMotionCatalog } from '../../../../avatar-runtime-vrm/src/runtime/motion-catalog'
import { buildKnownMotionInfo } from './vrm-animation-scanner'

describe('vrm motion catalog sync', () => {
  it('keeps the UI scanner catalog playable by the runtime catalog', () => {
    const scannerMotions = buildKnownMotionInfo()
    const runtimeMotions = getBuiltinMotionCatalog()

    expect(runtimeMotions.map((motion) => motion.id)).toEqual(scannerMotions.map((motion) => motion.id))

    for (const scannerMotion of scannerMotions) {
      const runtimeMotion = runtimeMotions.find((motion) => motion.id === scannerMotion.id)
      expect(runtimeMotion).toMatchObject({
        id: scannerMotion.id,
        kind: scannerMotion.kind,
        source: scannerMotion.path,
      })
    }
  })

  it('registers newly added idle and preview motions with the expected kinds', () => {
    const motions = new Map(getBuiltinMotionCatalog().map((motion) => [motion.id, motion]))

    expect(motions.get('LookAround')?.kind).toBe('idle')
    expect(motions.get('Relax')?.kind).toBe('idle')
    expect(motions.get('Sleepy')?.kind).toBe('idle')
    expect(motions.get('waiting')?.kind).toBe('idle')
    expect(motions.get('appearing')?.kind).toBe('oneshot')
    expect(motions.get('Clapping')?.kind).toBe('oneshot')
    expect(motions.get('Goodbye')?.kind).toBe('oneshot')
    expect(motions.get('liked')?.kind).toBe('oneshot')
    expect(motions.get('Thinking')?.kind).toBe('oneshot')
  })
})
