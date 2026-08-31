import { describe, expect, it } from 'vitest'
import type { ChannelEnvelopeV1 } from '../../protocol/neuro-link-v1'
import type { ProjectionEnvelopeEvent } from './client'
import {
  createNeuroLinkProjectionState,
  reduceNeuroLinkProjection,
} from './projection'

function event(
  sequence: number,
  payload: ChannelEnvelopeV1['payload'],
  source: 'live' | 'replay' = 'live',
): ProjectionEnvelopeEvent {
  return {
    method: payload.kind === 'presentation' ? 'presentation/event' : 'tool/lifecycle',
    cursor: { stream: 'session-1', sequence },
    source,
    envelope: {
      schema_version: 1,
      envelope_id: `envelope-${sequence}`,
      occurred_at: '2026-08-31T00:00:00.000Z',
      direction: 'internal_projection',
      address: { channel: 'neuro-link', chat_id: 'chat-1' },
      correlation: {
        session_key: 'session-1',
        request_id: 'request-1',
        trace_id: 'trace-1',
        sequence,
      },
      origin: 'runtime',
      payload,
      extensions: {},
    },
  }
}

describe('Neuro-Link projection reducer', () => {
  it('keeps replay subtitles visible without replaying them to TTS', () => {
    const initial = createNeuroLinkProjectionState('session-1')
    const replayed = reduceNeuroLinkProjection(initial, event(1, {
      kind: 'presentation',
      event: 'subtitle.updated',
      body: { text: 'restored answer' },
    }, 'replay'))
    expect(replayed.presentation.subtitle).toBe('restored answer')
    expect(replayed.presentation.shouldSpeak).toBe(false)

    const live = reduceNeuroLinkProjection(replayed, event(2, {
      kind: 'presentation',
      event: 'subtitle.updated',
      body: { text: 'new answer' },
    }))
    expect(live.presentation.subtitle).toBe('new answer')
    expect(live.presentation.shouldSpeak).toBe(true)

    const cleared = reduceNeuroLinkProjection(live, event(3, {
      kind: 'presentation',
      event: 'subtitle.cleared',
      body: {},
    }))
    expect(cleared.presentation.subtitle).toBe('')
    expect(cleared.presentation.shouldSpeak).toBe(false)
  })

  it('tracks semantic thinking, speaking, and tool lifecycle state', () => {
    let state = createNeuroLinkProjectionState('session-1')
    state = reduceNeuroLinkProjection(state, event(1, {
      kind: 'presentation',
      event: 'assistant.thinking.started',
      body: {},
    }))
    expect(state.presentation.thinking).toBe(true)
    state = reduceNeuroLinkProjection(state, event(2, {
      kind: 'presentation',
      event: 'assistant.thinking.completed',
      body: {},
    }))
    state = reduceNeuroLinkProjection(state, event(3, {
      kind: 'presentation',
      event: 'assistant.tool.started',
      body: { call_id: 'call-1', name: 'shell' },
    }))
    expect(state.presentation.tools['call-1']?.status).toBe('running')
    state = reduceNeuroLinkProjection(state, event(4, {
      kind: 'presentation',
      event: 'assistant.tool.completed',
      body: { call_id: 'call-1', name: 'shell', is_error: false },
    }))
    expect(state.presentation.tools['call-1']?.status).toBe('success')
  })
})
