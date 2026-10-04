/**
 * DN-4D — vivy-cognitive projection controller tests.
 *
 * Scenario coverage per plan: human setup gate, CAS conflict keeps draft,
 * unknown write is never replayed, current-vs-frozen revisions stay separate,
 * scope/receipt states surface verbatim.
 */
import { describe, expect, it, vi } from 'vitest';
import type { CapabilityStatus, CognitiveOutcome } from '../api/cognitive';
import { createVivyCognitiveController } from './vivy-cognitive';

function statusOk(over: Partial<CapabilityStatus> = {}): CapabilityStatus {
  return {
    profile_id: 'p',
    destination_id: 'd',
    persona: { state: 'ready', current_revisions: { identity: 1 } },
    frozen: { state: 'not_captured' },
    memory: {
      backend_id: 'mentle',
      health: 'available',
      reason_code: '',
      canonical_state: 'ready',
      index_state: 'ready',
    },
    cognition: {
      enabled: false,
      policy_revision: 0,
      min_interval_ms: 0,
      active_run_id: '',
      source_id: '',
      watermark: 0,
      pending_through: 0,
      phase: 'disabled',
      block_reason: '',
    },
    ...over,
  };
}

function ok<T>(value: T): CognitiveOutcome<T> {
  return { status: 'ok', value };
}
function failed<T>(code: string, retryable = false, value?: T): CognitiveOutcome<T> {
  return { status: 'failed', error: { code, message: code, retryable }, value };
}
function unavailable<T>(code = 'capability_unavailable'): CognitiveOutcome<T> {
  return { status: 'unavailable', error: { code, message: code, retryable: true } };
}
function unknownOutcome<T>(): CognitiveOutcome<T> {
  return {
    status: 'unknown',
    error: { code: 'outcome_unknown', message: 'call timed out', retryable: false },
  };
}

type Handler = (input: Record<string, unknown>) => CognitiveOutcome<unknown>;

function fakeClient(handlers: Record<string, Handler | CognitiveOutcome<unknown>>) {
  const call = vi.fn(async (action: string, input: Record<string, unknown>) => {
    const h = handlers[action];
    if (!h) throw new Error(`unexpected action ${action}`);
    return typeof h === 'function' ? h(input) : h;
  });
  return { call };
}

describe('vivy-cognitive controller', () => {
  it('humanSetupGate: uninitialized status gates send; initialize via bound session clears it', async () => {
    let personaState: 'uninitialized' | 'ready' = 'uninitialized';
    const client = fakeClient({
      status: () => ok(statusOk({ persona: { state: personaState, current_revisions: {} } })),
      'persona.initialize': () => {
        personaState = 'ready';
        return ok({ document: { kind: 'identity', revision: 1 }, changed: true });
      },
    });
    const c = createVivyCognitiveController({ client });
    await c.bind('sess-1');
    expect(c.needsSetup.value).toBe(true);
    expect(c.capabilityBlocked.value).toBe(false);

    const out = await c.initialize({
      identity: 'i',
      relationship: 'r',
      redline: 'x',
      user: 'u',
      world: 'w',
    });
    expect(out.status).toBe('ok');
    expect(client.call).toHaveBeenCalledWith(
      'persona.initialize',
      expect.objectContaining({ session_id: 'sess-1' }),
    );
    await vi.waitFor(() => expect(c.needsSetup.value).toBe(false));
  });

  it('casConflictRetainsDraft: failed revision_conflict keeps draft and refetches authority', async () => {
    const client = fakeClient({
      status: ok(statusOk()),
      'persona.read': ok({
        kind: 'identity',
        file_name: 'IDENTITY.MD',
        exists: true,
        valid: true,
        content: 'current-from-authority',
        revision: 3,
        content_hash: 'sha256:x',
        updated_at: 't',
        pending_count: 0,
      }),
      'persona.save': failed('persona_revision_conflict'),
    });
    const c = createVivyCognitiveController({ client });
    await c.bind('sess-1');
    await c.readPersona('identity');
    c.setDraft('identity', 'my edited draft');
    const out = await c.savePersona('identity', 'why');
    expect(out.status).toBe('failed');
    expect(c.drafts.identity).toBe('my edited draft');
    expect(c.personaErrors.value.identity?.code).toBe('persona_revision_conflict');
    // Authoritative readback refreshed the current document.
    expect(c.personaDocs.value.identity?.content).toBe('current-from-authority');
    expect(c.personaDocs.value.identity?.revision).toBe(3);
  });

  it('unknownSaveNoReplay: unknown write surfaces once, draft kept, reconcile is explicit', async () => {
    const saveCalls: Array<Record<string, unknown>> = [];
    const client = fakeClient({
      status: ok(statusOk()),
      'persona.read': ok({
        kind: 'identity',
        file_name: 'IDENTITY.MD',
        exists: true,
        valid: true,
        content: 'server truth',
        revision: 2,
        content_hash: 'sha256:y',
        updated_at: 't',
        pending_count: 0,
      }),
      'persona.save': (input) => {
        saveCalls.push(input);
        return unknownOutcome();
      },
    });
    const c = createVivyCognitiveController({ client });
    await c.bind('sess-1');
    await c.readPersona('identity');
    c.setDraft('identity', 'ambiguous draft');
    const out = await c.savePersona('identity', 'why');
    expect(out.status).toBe('unknown');
    expect(c.unknownSaves.value.identity).toBeTruthy();
    expect(c.drafts.identity).toBe('ambiguous draft');
    expect(saveCalls).toHaveLength(1); // no automatic replay

    await c.reconcilePersona('identity');
    expect(c.personaDocs.value.identity?.content).toBe('server truth');
    expect(c.personaDocs.value.identity?.revision).toBe(2);
    expect(saveCalls).toHaveLength(1); // reconcile is a read, not a save
  });

  it('currentVersusFrozen: current persona revisions and frozen revisions never mix', async () => {
    const client = fakeClient({
      status: ok(
        statusOk({
          persona: { state: 'ready', current_revisions: { identity: 5 } },
          frozen: { state: 'ready', revisions: { identity: 3 } },
        }),
      ),
      'frozen.read': ok({ state: 'ready', revisions: { identity: 3, user: 1 }, sections: {} }),
    });
    const c = createVivyCognitiveController({ client });
    await c.bind('sess-1');
    await c.readFrozen();
    expect(c.currentRevisionOf('identity')).toBe(5);
    expect(c.frozen.value?.revisions?.identity).toBe(3);
    expect(c.status.value?.persona.current_revisions.identity).toBe(5);
    expect(c.status.value?.frozen.revisions?.identity).toBe(3);
  });

  it('scopeAndReceiptStates: mutation scope/destination and receipt phases surface verbatim', async () => {
    const receipt = {
      operation_id: 'op-1',
      payload_digest: 'sha256:z',
      status: 'submitted',
      target_ref: 'card:9',
      revision: 0,
      error_code: '',
      canonical_status: 'pending',
      index_status: 'index_pending',
    };
    const client = fakeClient({
      status: ok(
        statusOk({
          scope: { user: 'u', lane: 'personal' },
          destination_id: 'dest-1',
        }),
      ),
      'memory.mutate': ok(receipt),
      'memory.receipt': ok({ ...receipt, status: 'accepted', canonical_status: 'applied' }),
    });
    const c = createVivyCognitiveController({ client });
    await c.bind('sess-1');
    const out = await c.mutateMemory({ operation: 'put', record_id: 'r1', body: '{}' });
    expect(out.status).toBe('ok');
    expect(c.lastMutation.value?.status).toBe('submitted');
    expect(c.lastMutation.value?.canonical_status).toBe('pending');
    expect(c.lastMutation.value?.index_status).toBe('index_pending');

    const r = await c.receipt('op-1');
    expect(r.status).toBe('ok');
    expect(c.receipts.value['op-1']?.canonical_status).toBe('applied');
    expect(c.status.value?.scope).toEqual({ user: 'u', lane: 'personal' });
    expect(c.status.value?.destination_id).toBe('dest-1');
  });

  it('unavailable capability leaves the view navigable with an honest block', async () => {
    const client = fakeClient({ status: unavailable() });
    const c = createVivyCognitiveController({ client });
    await c.bind('sess-1');
    expect(c.capabilityBlocked.value).toBe(true);
    expect(c.blockReason.value?.code).toBe('capability_unavailable');
    expect(c.needsSetup.value).toBe(false);
    expect(c.status.value).toBeNull();
  });

  it('stale responses from a previous session are discarded', async () => {
    let resolveSlow: ((v: CognitiveOutcome<unknown>) => void) | null = null;
    let statusCalls = 0;
    const client = {
      call: vi.fn((action: string) => {
        if (action === 'status') {
          statusCalls++;
          if (statusCalls === 1)
            return new Promise<CognitiveOutcome<unknown>>((r) => (resolveSlow = r));
          return Promise.resolve(ok(statusOk({ profile_id: 'sess-2-status' })));
        }
        return Promise.resolve(ok({}));
      }),
    };
    const c = createVivyCognitiveController({ client });
    const p1 = c.bind('sess-1');
    await c.bind('sess-2');
    resolveSlow!(ok(statusOk({ profile_id: 'sess-1-status' })));
    await p1;
    expect(c.sessionId.value).toBe('sess-2');
    // sess-1 status must not have landed on the sess-2 view.
    expect(c.status.value?.profile_id).toBe('sess-2-status');
  });
});
