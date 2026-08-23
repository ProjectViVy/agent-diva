import { describe, expect, it } from 'vitest'
import { GUI_CAPABILITIES } from './capabilities'

describe('G0 GUI capability ledger', () => {
  it('has unique, decision-complete entries for every capability class', () => {
    const ids = GUI_CAPABILITIES.map((capability) => capability.id)
    expect(new Set(ids).size).toBe(ids.length)
    expect(new Set(GUI_CAPABILITIES.map((capability) => capability.classification))).toEqual(
      new Set(['MANAGER', 'LOCAL', 'DEFERRED', 'REMOVED']),
    )

    for (const capability of GUI_CAPABILITIES) {
      expect(capability.entrypoint).not.toBe('')
      expect(capability.authority).not.toBe('')
      expect(capability.failure).not.toBe('')
      expect(capability.verification).not.toBe('')
    }
  })

  it('keeps domain authority behind Manager and host concerns local', () => {
    const byId = new Map(GUI_CAPABILITIES.map((capability) => [capability.id, capability]))

    expect(byId.get('chat.turn')).toMatchObject({
      classification: 'MANAGER',
      transport: 'tauri-manager-proxy',
    })
    expect(byId.get('plan.lifecycle')).toMatchObject({
      classification: 'MANAGER',
      transport: 'tauri-manager-proxy',
    })
    expect(byId.get('gateway.process')).toMatchObject({
      classification: 'LOCAL',
      transport: 'tauri-local',
    })
    expect(byId.get('desktop.mate')).toMatchObject({
      classification: 'LOCAL',
      transport: 'tauri-local',
    })
  })

  it('gives unavailable and removed capabilities no transport', () => {
    for (const capability of GUI_CAPABILITIES.filter(
      ({ classification }) => classification === 'DEFERRED' || classification === 'REMOVED',
    )) {
      expect(capability.entrypoint).toBe('none')
      expect(capability.transport).toBe('none')
    }
  })
})
