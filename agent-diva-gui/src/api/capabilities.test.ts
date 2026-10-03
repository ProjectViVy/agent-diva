import { describe, expect, it } from 'vitest'
import {
  GUI_CAPABILITIES,
  markCapabilityFailed,
  negotiateCapabilities,
} from './capabilities'

const VIVY_CAPS = [
  'session',
  'turn',
  'run',
  'approval',
  'question',
  'plan',
  'commands.list',
  'settings.get',
  'settings.providers',
  'settings.mcp',
  'skills.list',
  'cron.list',
  'session.todos',
  'stats.tokens',
]

describe('DN-1 capability ledger', () => {
  it('has unique entries with complete dispositions', () => {
    const ids = GUI_CAPABILITIES.map((c) => c.id)
    expect(new Set(ids).size).toBe(ids.length)
    for (const c of GUI_CAPABILITIES) {
      expect(c.authority).not.toBe('')
      expect(c.failure).not.toBe('')
      expect(c.verification).not.toBe('')
      if (c.disposition === 'existing-backend') {
        expect(c.vivyCapability).toBeTruthy()
        expect(c.entrypoint).not.toBe('none')
      } else {
        expect(c.vivyCapability).toBeNull()
      }
    }
  })

  it('resolves available/absent/disconnected from the negotiated set', () => {
    const state = negotiateCapabilities(VIVY_CAPS, true)
    expect(state.get('chat.turn')).toBe('available')
    expect(state.get('session.lifecycle')).toBe('available')
    expect(state.get('command.approval')).toBe('available')
    // backend gaps and retired surfaces are absent, never faked
    expect(state.get('audit.logs')).toBe('absent')
    expect(state.get('mask.lifecycle')).toBe('absent')
    expect(state.get('memory.home')).toBe('absent')
    expect(state.get('gateway.process')).toBe('absent')
    expect(state.get('memory.laputa')).toBe('absent')
    // native ops stay available regardless of negotiation
    expect(state.get('desktop.preferences')).toBe('available')
  })

  it('marks backend entries disconnected when transport is down', () => {
    const state = negotiateCapabilities(VIVY_CAPS, false)
    expect(state.get('chat.turn')).toBe('disconnected')
    expect(state.get('session.lifecycle')).toBe('disconnected')
    // gaps/retired stay absent — disconnected is not a disguise for absent
    expect(state.get('audit.logs')).toBe('absent')
    // native ops unaffected by transport
    expect(state.get('desktop.pet')).toBe('available')
  })

  it('reports absent for a negotiated-missing capability', () => {
    const state = negotiateCapabilities(
      VIVY_CAPS.filter((c) => c !== 'approval'),
      true,
    )
    expect(state.get('command.approval')).toBe('absent')
    expect(state.get('chat.turn')).toBe('available')
  })

  it('marks failed after a live call error', () => {
    const state = negotiateCapabilities(VIVY_CAPS, true)
    const next = markCapabilityFailed(state, 'providers')
    expect(next.get('providers')).toBe('failed')
    expect(state.get('providers')).toBe('available') // original untouched
  })
})
