import { flushPromises, shallowMount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import DiagnosticsPanel from './DiagnosticsPanel.vue';
import { DiagnosticsReader, GuiDiagnosticRecorder } from '../../state/gui-diagnostics';
import { VivyClient } from '../../api/vivy/client';
import type { VivyTransport } from '../../api/vivy/transport';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) =>
      ({
        'diagnostics.title': 'Diagnostics',
        'diagnostics.desc': 'Bounded runtime and GUI log tails',
        'diagnostics.sourceRuntime': 'Runtime logs',
        'diagnostics.sourceGui': 'GUI logs',
        'diagnostics.allLevels': 'All levels',
        'diagnostics.queryPlaceholder': 'Filter…',
        'diagnostics.refresh': 'Refresh',
        'diagnostics.nextPage': 'Next page',
        'diagnostics.gap': 'Log rotation detected — earlier records may be missing',
        'diagnostics.hasMore': 'More records exist beyond the bound',
        'diagnostics.truncated': 'clipped',
        'diagnostics.queued': 'queued',
        'diagnostics.persisted': 'persisted',
        'diagnostics.dropped': 'dropped',
        'diagnostics.unconfirmed': 'unconfirmed',
        'diagnostics.appendError': 'append error',
        'diagnostics.loading': 'Loading…',
        'diagnostics.empty': 'No records',
      } as Record<string, string>)[key] ?? key,
  }),
}));

type CallHandler = (params: unknown) => unknown | Promise<unknown>;

function makeClient(handlers: Record<string, CallHandler>) {
  const transport: VivyTransport = {
    call: (req) => {
      const handler = handlers[req.method]
      if (!handler) return Promise.reject(new Error(`unexpected call ${req.method}`))
      return Promise.resolve(handler(req.params))
    },
    onEvent: () => Promise.resolve(() => {}),
    close: () => {},
  }
  return new VivyClient(transport)
}

function makePanel(handlers: Record<string, CallHandler>, recorder?: GuiDiagnosticRecorder) {
  const client = makeClient(handlers)
  return shallowMount(DiagnosticsPanel, {
    props: {
      reader: new DiagnosticsReader(client),
      recorder: recorder ?? new GuiDiagnosticRecorder(client),
    },
  })
}

describe('DiagnosticsPanel', () => {
  it('renders records verbatim from the bounded page', async () => {
    const wrapper = makePanel({
      'diagnostics/logs': () => ({
        source: 'runtime',
        records: [
          { id: '2026-10-03:0', at: 1791030687631, level: 'info', component: 'dn0', message: 'probe ok', truncated: false },
        ],
        gap: false,
        has_more: false,
      }),
    })
    await flushPromises()
    expect(wrapper.text()).toContain('probe ok')
    expect(wrapper.text()).not.toContain('No records')
  })

  it('shows the rotation gap banner and next-page affordance', async () => {
    const wrapper = makePanel({
      'diagnostics/logs': () => ({
        source: 'gui',
        records: [
          { id: '2026-10-03:5', level: 'warn', component: 'x', message: 'late row', truncated: true },
        ],
        next_cursor: 'cur-9',
        gap: true,
        has_more: true,
      }),
    })
    await flushPromises()
    const text = wrapper.text()
    expect(text).toContain('Log rotation detected')
    expect(text).toContain('More records exist')
    expect(text).toContain('clipped')
    const next = wrapper.findAll('button').find((b) => b.text() === 'Next page')
    expect(next?.attributes('disabled')).toBeUndefined()
  })

  it('shows recorder loss accounting — dropped and unconfirmed stay visible', async () => {
    const client = makeClient({
      'diagnostics/logs': () => ({ source: 'runtime', records: null, gap: false, has_more: false }),
      'diagnostics/gui/append': () => Promise.reject(new Error('disk full')),
    })
    const recorder = new GuiDiagnosticRecorder(client)
    for (let i = 0; i < 5; i++) recorder.record({ message: `x-${i}` })
    await recorder.flush()
    const wrapper = makePanel(
      { 'diagnostics/logs': () => ({ source: 'runtime', records: null, gap: false, has_more: false }) },
      recorder,
    )
    await flushPromises()
    const text = wrapper.text()
    expect(text).toContain('unconfirmed: 5')
    expect(text).toContain('append error')
  })

  it('renders empty state honestly when records is null', async () => {
    const wrapper = makePanel({
      'diagnostics/logs': () => ({ source: 'runtime', records: null, gap: false, has_more: false }),
    })
    await flushPromises()
    expect(wrapper.text()).toContain('No records')
  })

  it('does not claim there are no records when loading diagnostics failed', async () => {
    const wrapper = makePanel({
      'diagnostics/logs': () => Promise.reject(new Error('diagnostics unavailable')),
    })
    await flushPromises()
    expect(wrapper.text()).toContain('diagnostics unavailable')
    expect(wrapper.text()).not.toContain('No records')
  })
})
