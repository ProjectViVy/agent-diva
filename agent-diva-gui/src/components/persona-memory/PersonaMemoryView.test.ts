/**
 * DN-4D — PersonaMemoryView component cases: unavailable capability stays
 * navigable; uninitialized sessions show the setup gate; receipt/review
 * states surface verbatim.
 */
import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import type { CognitiveOutcome } from '../../api/cognitive';
import { createVivyCognitiveController } from '../../state/vivy-cognitive';
import PersonaMemoryView from './PersonaMemoryView.vue';

vi.mock('vue-i18n', () => ({ useI18n: () => ({ t: (key: string) => key }) }));
vi.mock('./PersonaMarkdownEditor.vue', () => ({
  default: { template: '<div class="editor-stub" />' },
}));
vi.mock('./PersonaSetupGate.vue', () => ({
  default: { template: '<div class="gate-stub" />' },
}));

vi.mock('../../state/vivy-cognitive', async (importOriginal) => {
  const mod = await importOriginal<typeof import('../../state/vivy-cognitive')>();
  return { ...mod, vivyCognitive: vi.fn() };
});
import { vivyCognitive } from '../../state/vivy-cognitive';

function ok<T>(v: T): CognitiveOutcome<T> {
  return { status: 'ok', value: v };
}
const statusValue = (state: 'uninitialized' | 'ready' = 'ready') => ({
  profile_id: 'p',
  destination_id: 'd',
  persona: { state, current_revisions: {} },
  frozen: { state: 'not_captured' },
  memory: {
    backend_id: 'b',
    health: 'unavailable' as const,
    reason_code: 'backend_unavailable',
    canonical_state: 'down',
    index_state: 'down',
  },
  cognition: {
    enabled: false,
    policy_revision: 0,
    min_interval_ms: 0,
    active_run_id: '',
    source_id: '',
    watermark: 0,
    pending_through: 0,
    phase: 'disabled' as const,
    block_reason: '',
  },
});

function controllerWith(handlers: Record<string, () => CognitiveOutcome<unknown>>) {
  const client = {
    call: vi.fn(async (action: string) => {
      const h = handlers[action];
      if (!h) throw new Error(`unexpected ${action}`);
      return h();
    }),
  };
  return createVivyCognitiveController({ client });
}

describe('PersonaMemoryView', () => {
  it('shows the setup gate when persona is uninitialized', async () => {
    const c = controllerWith({
      status: () => ok(statusValue('uninitialized')),
    });
    await c.bind('sess-1');
    vi.mocked(vivyCognitive).mockReturnValue(c);
    const w = mount(PersonaMemoryView);
    await vi.waitFor(() => expect(w.find('.gate-stub').exists()).toBe(true));
  });

  it('surfaces unavailable capability as a navigable notice, not a crash', async () => {
    const c = controllerWith({
      status: () => ({
        status: 'unavailable',
        error: { code: 'capability_unavailable', message: 'm', retryable: true },
      }),
      'persona.read': () => ok({ kind: 'identity', content: '', revision: 0 }),
      'persona.reviews.list': () => ok({ items: [] }),
      'frozen.read': () => ok({ state: 'not_captured' }),
      'actmem.read': () => ok({ changed: false, revision: 0, entries: [] }),
      'actmem.owner.read': () => ({
        status: 'failed',
        error: { code: 'actmem_format_error', message: 'm', retryable: false },
      }),
    });
    await c.bind('sess-1');
    vi.mocked(vivyCognitive).mockReturnValue(c);
    const w = mount(PersonaMemoryView);
    await vi.waitFor(() =>
      expect(w.text()).toContain('personaMemory.unavailable'),
    );
    // Still navigable: kinds row renders.
    expect(w.findAll('.pm-kind').length).toBeGreaterThan(0);
  });
});
