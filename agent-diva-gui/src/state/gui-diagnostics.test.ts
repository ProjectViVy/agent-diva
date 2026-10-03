import { describe, expect, it, vi } from 'vitest'
import type { WireEvent } from '../api/vivy/contracts'
import { VivyClient } from '../api/vivy/client'
import type { VivyTransport } from '../api/vivy/transport'
import {
  DiagnosticsReader,
  GuiDiagnosticRecorder,
  sanitizeFields,
  sanitizeMessage,
  GUI_QUEUE_MAX_RECORDS,
} from './gui-diagnostics'

type CallHandler = (params: unknown) => unknown | Promise<unknown>

function makeClient(handlers: Record<string, CallHandler>) {
  const calls: Array<{ method: string; params: unknown }> = []
  const transport: VivyTransport = {
    call: (req) => {
      calls.push({ method: req.method, params: req.params })
      const handler = handlers[req.method]
      if (!handler) return Promise.reject(new Error(`unexpected call ${req.method}`))
      return Promise.resolve(handler(req.params))
    },
    onEvent: () => Promise.resolve(() => {}),
    close: () => {},
  }
  return { client: new VivyClient(transport), calls }
}

function guiAppendCalls(calls: Array<{ method: string; params: unknown }>) {
  return calls.filter((c) => c.method === 'diagnostics/gui/append')
}

describe('GuiDiagnosticRecorder', () => {
  it('partialAppendUnknownNoRetry: ambiguous write moves batch to unconfirmed, never retries', async () => {
    const h = makeClient({
      'diagnostics/gui/append': () =>
        Promise.reject(new Error('logging: gui append failed after 12 accepted record(s): disk full')),
    })
    const rec = new GuiDiagnosticRecorder(h.client)
    for (let i = 0; i < 20; i++) rec.record({ message: `m-${i}` })
    await rec.flush()
    const stats = rec.stats()
    // Nothing is counted as persisted; the whole dispatched batch is
    // unconfirmed — the accepted-prefix number in the error text is
    // deliberately NOT parsed.
    expect(stats.persisted).toBe(0)
    expect(stats.unconfirmed).toBe(20)
    expect(stats.lastError).toContain('disk full')
    // A second flush attempt is a no-op (queue drained); no retry storm.
    await rec.flush()
    expect(guiAppendCalls(h.calls)).toHaveLength(1)
  })

  it('ack counts only the accepted prefix; remainder is unconfirmed', async () => {
    const h = makeClient({
      'diagnostics/gui/append': () => ({ accepted: 3 }),
    })
    const rec = new GuiDiagnosticRecorder(h.client)
    for (let i = 0; i < 5; i++) rec.record({ message: `m-${i}` })
    await rec.flush()
    expect(rec.stats().persisted).toBe(3)
    expect(rec.stats().unconfirmed).toBe(2)
  })

  it('queueBoundsAndDropCount: cap is 1000 records / ~1MiB; drops are counted', async () => {
    const h = makeClient({
      'diagnostics/gui/append': () => Promise.resolve({ accepted: 50 }),
    })
    const rec = new GuiDiagnosticRecorder(h.client)
    for (let i = 0; i < GUI_QUEUE_MAX_RECORDS + 50; i++) {
      rec.record({ message: `m-${i}` })
    }
    const stats = rec.stats()
    expect(stats.queued).toBe(GUI_QUEUE_MAX_RECORDS)
    expect(stats.dropped).toBe(50)
    // Batches never exceed 50 records each.
    await rec.flush()
    const batches = guiAppendCalls(h.calls)
    expect(batches.length).toBeGreaterThan(0)
    for (const call of batches) {
      const records = (call.params as { records: unknown[] }).records
      expect(records.length).toBeLessThanOrEqual(50)
    }
    // All 1000 records reached the wire across batches.
    expect(batches.reduce((n, c) => n + (c.params as { records: unknown[] }).records.length, 0)).toBe(1000)
  })

  it('flush is single-flight: concurrent schedules produce one drain', async () => {
    const h = makeClient({
      'diagnostics/gui/append': () => ({ accepted: 50 }),
    })
    const rec = new GuiDiagnosticRecorder(h.client)
    for (let i = 0; i < 10; i++) rec.record({ message: `m-${i}` })
    await Promise.all([rec.flush(), rec.flush(), rec.flush()])
    expect(rec.stats().persisted).toBe(10)
  })

  it('speechSentinelRedacted: secret/text/audio/body/url never reach the wire', async () => {
    let captured: unknown = null
    const h = makeClient({
      'diagnostics/gui/append': (params) => {
        captured = params
        return { accepted: 1 }
      },
    })
    const rec = new GuiDiagnosticRecorder(h.client)
    rec.record({
      component: 'speech',
      level: 'debug',
      message: 'stt used api_key=sk-ABCDEF0123456789 and Bearer tok12345 at https://provider.example.com/some/very/long/path/segment/still-going',
      fields: {
        provider: 'siliconflow',
        audio_url: 'https://media.example.com/very/long/path/hidden',
        pcm_body: 'QUJD' + 'A'.repeat(200),
        transcript_text: 'user said secret words',
        run_id: 'run_x',
        attempts: 2,
      },
    })
    await rec.flush()
    const wire = JSON.stringify(captured)
    expect(wire).not.toContain('sk-ABCDEF0123456789')
    expect(wire).not.toContain('tok12345')
    expect(wire).not.toContain('provider.example.com')
    expect(wire).not.toContain('media.example.com')
    expect(wire).not.toContain('pcm_body')
    expect(wire).not.toContain('transcript_text')
    expect(wire).not.toContain('secret words')
    // Allowlisted scalars survive.
    expect(wire).toContain('siliconflow')
    expect(wire).toContain('run_x')
    expect(wire).toContain('⟨redacted⟩')
  })

  it('sanitizers bound message length and drop non-scalar fields', () => {
    expect(sanitizeMessage('x'.repeat(2000)).length).toBeLessThanOrEqual(1025)
    const fields = sanitizeFields({
      ok: 'yes',
      nested: { a: 1 },
      token_hint: 'abc',
      n: 42,
      flag: true,
    })
    expect(fields).toEqual({ ok: 'yes', n: 42, flag: true })
    expect(fields).not.toHaveProperty('nested')
    expect(fields).not.toHaveProperty('token_hint')
  })
})

describe('DiagnosticsReader', () => {
  it('readerErrorNoRecursion: a read failure surfaces lastError and enqueues nothing', async () => {
    const rec = new GuiDiagnosticRecorder(
      makeClient({ 'diagnostics/gui/append': () => ({ accepted: 1 }) }).client,
    )
    const h = makeClient({
      'diagnostics/logs': () => Promise.reject(new Error('logging: invalid diagnostic query: unknown source "bogus"')),
      'diagnostics/gui/append': () => ({ accepted: 1 }),
    })
    const reader = new DiagnosticsReader(h.client)
    reader.filter = { source: 'bogus' }
    const page = await reader.refresh()
    expect(page).toBeNull()
    expect(reader.lastError).toContain('unknown source')
    expect(rec.stats().queued).toBe(0)
    expect(guiAppendCalls(h.calls)).toHaveLength(0)
  })

  it('rotationGapVisible: gap flag stays verbatim on the retained page', async () => {
    const h = makeClient({
      'diagnostics/logs': () => ({
        source: 'gui',
        records: [{ id: '2026-10-03:0', level: 'info', component: 'dn0', message: 'x', truncated: false }],
        next_cursor: 'cur-1',
        gap: true,
        has_more: true,
      }),
    })
    const reader = new DiagnosticsReader(h.client)
    const page = await reader.refresh({ source: 'gui', date: '2026-10-03' })
    expect(page?.gap).toBe(true)
    expect(page?.has_more).toBe(true)
    expect(page?.records).toHaveLength(1)
    // next_page uses the server-issued cursor verbatim.
    const calls = h.calls.filter((c) => c.method === 'diagnostics/logs')
    await reader.nextPage()
    const calls2 = h.calls.filter((c) => c.method === 'diagnostics/logs')
    expect(calls2.length).toBe(calls.length + 1)
    expect((calls2[calls2.length - 1].params as { after: string }).after).toBe('cur-1')
  })

  it('refresh without has_more does not page', async () => {
    const h = makeClient({
      'diagnostics/logs': () => ({
        source: 'runtime',
        records: null,
        gap: false,
        has_more: false,
      }),
    })
    const reader = new DiagnosticsReader(h.client)
    await reader.refresh()
    expect(await reader.nextPage()).toBeNull()
  })
})

describe('recordGuiDiagnostic transport', () => {
  it('batch guard rejects >50 records without calling the wire', async () => {
    const { appendGuiLogs, GUI_BATCH_MAX_RECORDS } = await import('../api/vivy/observability')
    const h = makeClient({})
    await expect(
      appendGuiLogs(h.client, {
        records: Array.from({ length: GUI_BATCH_MAX_RECORDS + 1 }, (_, i) => ({ message: `m-${i}` })),
      }),
    ).rejects.toThrow('too large')
    expect(guiAppendCalls(h.calls)).toHaveLength(0)
  })
})
