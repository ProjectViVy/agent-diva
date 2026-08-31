import { describe, expect, it, vi } from 'vitest'
import {
  CHANNEL_SCHEMA_VERSION_V1,
  NEURO_LINK_PROTOCOL_V1,
  type ChannelEnvelopeV1,
  type ChannelPayloadV1,
  type ProjectionEventV1,
} from '../../protocol/neuro-link-v1'
import {
  NeuroLinkClient,
  type WebSocketEventLike,
  type WebSocketLike,
} from './client'

type Listener = (event: WebSocketEventLike) => void

class FakeWebSocket implements WebSocketLike {
  readonly sent: string[] = []
  private readonly listeners = new Map<string, Listener[]>()

  addEventListener(type: string, listener: Listener): void {
    this.listeners.set(type, [...(this.listeners.get(type) ?? []), listener])
  }

  send(data: string): void {
    this.sent.push(data)
  }

  close(): void {
    this.emit('close', {})
  }

  emit(type: string, event: WebSocketEventLike): void {
    for (const listener of this.listeners.get(type) ?? []) listener(event)
  }
}

function frame(socket: FakeWebSocket, method: string): Record<string, any> | undefined {
  return socket.sent
    .map((raw) => JSON.parse(raw) as Record<string, any>)
    .find((value) => value.method === method)
}

async function waitForFrame(socket: FakeWebSocket, method: string): Promise<Record<string, any>> {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    const value = frame(socket, method)
    if (value) return value
    await new Promise((resolve) => setTimeout(resolve, 0))
  }
  throw new Error(`timed out waiting for ${method}`)
}

async function waitForAck(socket: FakeWebSocket, sequence: number): Promise<Record<string, any>> {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    const value = socket.sent
      .map((raw) => JSON.parse(raw) as Record<string, any>)
      .find((frame) => frame.method === 'event/ack' && frame.params?.cursor?.sequence === sequence)
    if (value) return value
    await new Promise((resolve) => setTimeout(resolve, 0))
  }
  throw new Error(`timed out waiting for event/ack ${sequence}`)
}

async function flush(): Promise<void> {
  await Promise.resolve()
  await Promise.resolve()
}

function envelope(sequence: number, payload: ChannelPayloadV1): ChannelEnvelopeV1 {
  return {
    schema_version: CHANNEL_SCHEMA_VERSION_V1,
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
  }
}

function projection(sequence: number): ProjectionEventV1 {
  return {
    method: 'conversation/stream',
    cursor: { stream: 'session-1', sequence },
    envelope: envelope(sequence, {
      kind: 'stream',
      phase: 'delta',
      parts: [{ kind: 'text', text: `delta-${sequence}` }],
    }),
  }
}

async function answer(
  socket: FakeWebSocket,
  method: string,
  result: unknown,
): Promise<Record<string, any>> {
  const request = await waitForFrame(socket, method)
  socket.emit('message', {
    data: JSON.stringify({ jsonrpc: '2.0', id: request.id, result }),
  })
  await flush()
  return request
}

describe('NeuroLinkClient', () => {
  it('performs the typed hello/session handshake, replays, and ACKs projections', async () => {
    const socket = new FakeWebSocket()
    const onProjection = vi.fn()
    const client = new NeuroLinkClient({
      endpoint: () => 'ws://127.0.0.1:4312/api/neuro-link/v1/ws',
      frontendInstanceId: 'frontend-1',
      webSocketFactory: () => socket,
      onProjection,
      requestTimeoutMs: 500,
    })

    const connecting = client.connect()
    await flush()
    socket.emit('open', {})
    const hello = await waitForFrame(socket, 'protocol/hello')
    expect(hello.params).toMatchObject({
      protocol: NEURO_LINK_PROTOCOL_V1,
      schema_version: CHANNEL_SCHEMA_VERSION_V1,
      frontend_instance_id: 'frontend-1',
    })
    socket.emit('message', {
      data: JSON.stringify({
        jsonrpc: '2.0',
        id: hello.id,
        result: {
          protocol: NEURO_LINK_PROTOCOL_V1,
          schema_version: CHANNEL_SCHEMA_VERSION_V1,
          capabilities: ['state_sync'],
        },
      }),
    })
    await expect(connecting).resolves.toMatchObject({
      protocol: NEURO_LINK_PROTOCOL_V1,
      schema_version: CHANNEL_SCHEMA_VERSION_V1,
    })
    expect(client.connectionStatus).toBe('connected')

    const syncEvent = projection(1)
    const opening = client.openSession({
      session_key: 'session-1',
      durable_cursor: { stream: 'session-1', sequence: 0 },
    })
    await answer(socket, 'session/open', {
      opened: {
        session_key: 'session-1',
        cursor: { stream: 'session-1', sequence: 1 },
        catalog_revision: 1,
        services: [],
      },
      sync: {
        mode: 'replay',
        head: { stream: 'session-1', sequence: 1 },
        events: [syncEvent],
        snapshot: [],
      },
    })
    const replayAck = await waitForFrame(socket, 'event/ack')
    expect(replayAck.params.cursor).toEqual({ stream: 'session-1', sequence: 1 })
    socket.emit('message', {
      data: JSON.stringify({ jsonrpc: '2.0', id: replayAck.id, result: { cursor: replayAck.params.cursor } }),
    })
    await expect(opening).resolves.toBeDefined()
    expect(onProjection).toHaveBeenCalledTimes(1)
    expect(onProjection.mock.calls[0][0].source).toBe('replay')

    socket.emit('message', {
      data: JSON.stringify({
        jsonrpc: '2.0',
        method: 'conversation/stream',
        params: { envelope: envelope(2, {
          kind: 'stream',
          phase: 'delta',
          parts: [{ kind: 'text', text: 'live' }],
        }) },
      }),
    })
    const liveAck = await waitForAck(socket, 2)
    socket.emit('message', {
      data: JSON.stringify({ jsonrpc: '2.0', id: liveAck.id, result: { cursor: liveAck.params.cursor } }),
    })
    await flush()
    expect(onProjection).toHaveBeenCalledTimes(2)
    expect(onProjection.mock.calls[1][0].source).toBe('live')

    // A duplicate cursor is ignored before the application callback and ACK.
    socket.emit('message', {
      data: JSON.stringify({
        jsonrpc: '2.0',
        method: 'conversation/stream',
        params: { envelope: envelope(1, {
          kind: 'stream',
          phase: 'delta',
          parts: [{ kind: 'text', text: 'duplicate' }],
        }) },
      }),
    })
    await flush()
    expect(onProjection).toHaveBeenCalledTimes(2)

    socket.emit('message', {
      data: JSON.stringify({
        jsonrpc: '2.0',
        method: 'state/gap',
        params: { envelope: envelope(3, {
          kind: 'gap',
          reason: 'live subscriber lagged',
        }) },
      }),
    })
    const resume = await waitForFrame(socket, 'state/resume')
    expect(resume.params.cursor).toEqual({ stream: 'session-1', sequence: 2 })
    socket.emit('message', {
      data: JSON.stringify({
        jsonrpc: '2.0',
        id: resume.id,
        result: {
          mode: 'replay',
          head: { stream: 'session-1', sequence: 4 },
          events: [projection(4)],
          snapshot: [],
        },
      }),
    })
    for (let attempt = 0; attempt < 100 && onProjection.mock.calls.length < 3; attempt += 1) {
      await new Promise((resolve) => setTimeout(resolve, 0))
    }
    const resumeAck = await waitForAck(socket, 4)
    socket.emit('message', {
      data: JSON.stringify({ jsonrpc: '2.0', id: resumeAck.id, result: { cursor: resumeAck.params.cursor } }),
    })
    await flush()
    expect(onProjection).toHaveBeenCalledTimes(3)
    client.close()
  })

  it('rejects a handshake when the gateway returns a different schema version', async () => {
    const socket = new FakeWebSocket()
    const client = new NeuroLinkClient({
      endpoint: () => 'ws://127.0.0.1:4312/api/neuro-link/v1/ws',
      frontendInstanceId: 'frontend-1',
      webSocketFactory: () => socket,
      requestTimeoutMs: 500,
    })
    const connecting = client.connect()
    await flush()
    socket.emit('open', {})
    const hello = await waitForFrame(socket, 'protocol/hello')
    socket.emit('message', {
      data: JSON.stringify({
        jsonrpc: '2.0',
        id: hello.id,
        result: {
          protocol: NEURO_LINK_PROTOCOL_V1,
          schema_version: 99,
          capabilities: [],
        },
      }),
    })
    await expect(connecting).rejects.toThrow('protocol/schema mismatch')
    expect(client.connectionStatus).toBe('error')
  })
})
