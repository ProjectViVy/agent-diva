/**
 * DN-4D — vivy-cognitive projection controller.
 *
 * Session-scoped view of the sealed `vivy/diva-cognitive` module. Holds the
 * persona/actmem/memory/evolution surfaces behind one authoritative status
 * read per bound session.
 *
 * Rules (C2-3 + DN-4D plan):
 * - Session-scoped stale-response fencing: results arriving after a session
 *   change are discarded (epoch check before every commit).
 * - Save failures/timeouts keep the local draft; ambiguous (unknown) writes
 *   are surfaced and reconciled by explicit reads, never auto-replayed.
 * - Current persona revisions and frozen revisions are separate fields; the
 *   view never substitutes one for the other.
 * - A `unavailable` capability leaves the view navigable: status stays null,
 *   `capabilityBlocked` carries the typed error.
 * - No timers/background polling; refresh is explicit (bind/refresh calls).
 */
import { computed, reactive, ref, type ComputedRef, type Ref } from 'vue';
import { cognitiveClient } from '../api/cognitive';
import type {
  ActivityResult,
  CapabilityStatus,
  CognitiveClient,
  CognitiveErrorBody,
  CognitiveOutcome,
  CognitivePolicy,
  FrozenReadback,
  MemoryCardPage,
  MemoryEvidencePage,
  MutationReceipt,
  OwnerActmemDocument,
  PersonaDocument,
  PersonaInitialization,
  PersonaKind,
  PersonaReview,
  PersonaReviewPage,
  PersonaWriteOutcome,
  ResultsPage,
  TriggerResult,
  WorkPatch,
} from '../api/cognitive';

export interface CognitiveControllerDeps {
  client: CognitiveClient;
}

type Outcome<T> = CognitiveOutcome<T>;

export function createVivyCognitiveController(deps: CognitiveControllerDeps) {
  const { client } = deps;

  let epoch = 0;

  const sessionId: Ref<string | null> = ref(null);
  const loading = ref(false);
  const status: Ref<CapabilityStatus | null> = ref(null);
  /** Typed non-ok result of the last `status` call (capability blocked). */
  const blockReason: Ref<CognitiveErrorBody | null> = ref(null);
  /** Thrown transport/protocol error on the last `status` call. */
  const statusError: Ref<string | null> = ref(null);

  // Persona surface
  const personaDocs: Ref<Partial<Record<PersonaKind, PersonaDocument>>> = ref({});
  /** User-edited content per kind; survives save failures/timeouts. */
  const drafts = reactive<Record<string, string>>({});
  const personaErrors: Ref<Partial<Record<PersonaKind, CognitiveErrorBody>>> = ref({});
  /** Saves whose outcome is unknown — displayed until reconciled by read. */
  const unknownSaves: Ref<Partial<Record<PersonaKind, CognitiveErrorBody>>> = ref({});
  const reviews: Ref<PersonaReview[]> = ref([]);
  const reviewsCursor: Ref<string | undefined> = ref(undefined);

  // Frozen Core
  const frozen: Ref<FrozenReadback | null> = ref(null);

  // ACTMEM
  const actmem: Ref<ActivityResult | null> = ref(null);
  const ownerDoc: Ref<OwnerActmemDocument | null> = ref(null);
  const actmemError: Ref<CognitiveErrorBody | null> = ref(null);

  // Memory
  const memoryCards: Ref<MemoryCardPage | null> = ref(null);
  const evidence: Ref<MemoryEvidencePage | null> = ref(null);
  const memoryError: Ref<CognitiveErrorBody | null> = ref(null);
  const lastMutation: Ref<MutationReceipt | null> = ref(null);
  const receipts: Ref<Record<string, MutationReceipt>> = ref({});

  // Evolution / cognition
  const policy: Ref<CognitivePolicy | null> = ref(null);
  const policyError: Ref<CognitiveErrorBody | null> = ref(null);
  const lastTrigger: Ref<TriggerResult | null> = ref(null);
  const results: Ref<ResultsPage | null> = ref(null);

  const capabilityBlocked: ComputedRef<boolean> = computed(() => blockReason.value !== null);
  const needsSetup: ComputedRef<boolean> = computed(
    () =>
      status.value !== null &&
      (status.value.persona.state === 'uninitialized' ||
        status.value.persona.state === 'incomplete'),
  );
  const frozenRecoveryRequired: ComputedRef<boolean> = computed(
    () => status.value?.frozen.state === 'recovery_required',
  );

  function currentRevisionOf(kind: PersonaKind): number | undefined {
    return status.value?.persona.current_revisions[kind];
  }

  function clearSessionState() {
    status.value = null;
    blockReason.value = null;
    statusError.value = null;
    personaDocs.value = {};
    for (const k of Object.keys(drafts)) delete drafts[k];
    personaErrors.value = {};
    unknownSaves.value = {};
    reviews.value = [];
    reviewsCursor.value = undefined;
    frozen.value = null;
    actmem.value = null;
    ownerDoc.value = null;
    actmemError.value = null;
    memoryCards.value = null;
    evidence.value = null;
    memoryError.value = null;
    lastMutation.value = null;
    receipts.value = {};
    policy.value = null;
    policyError.value = null;
    lastTrigger.value = null;
    results.value = null;
  }

  /** Returns a fencing check: true while the captured epoch/session are live. */
  function fresh(e: number, sid: string): boolean {
    return e === epoch && sessionId.value === sid;
  }

  async function call<T>(
    sid: string,
    action: Parameters<CognitiveClient['call']>[0],
    input: Record<string, unknown>,
  ): Promise<Outcome<T>> {
    return client.call<T>(action, { session_id: sid, ...input });
  }

  async function bind(sid: string): Promise<void> {
    epoch++;
    sessionId.value = sid;
    clearSessionState();
    await refresh();
  }

  async function unbind(): Promise<void> {
    epoch++;
    sessionId.value = null;
    clearSessionState();
  }

  async function refresh(): Promise<Outcome<CapabilityStatus> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    loading.value = true;
    try {
      const out = await call<CapabilityStatus>(sid, 'status', {});
      if (!fresh(e, sid)) return null;
      if (out.status === 'ok') {
        status.value = out.value;
        blockReason.value = null;
        statusError.value = null;
      } else {
        status.value = null;
        blockReason.value = out.error;
        statusError.value = null;
      }
      return out;
    } catch (err) {
      if (!fresh(e, sid)) return null;
      status.value = null;
      blockReason.value = null;
      statusError.value = err instanceof Error ? err.message : String(err);
      return null;
    } finally {
      if (fresh(e, sid)) loading.value = false;
    }
  }

  // ---- persona ----

  async function initialize(
    initialization: PersonaInitialization,
    reason?: string,
  ): Promise<Outcome<PersonaWriteOutcome> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const input: Record<string, unknown> = { initialization };
    if (reason) input.reason = reason;
    const out = await call<PersonaWriteOutcome>(sid, 'persona.initialize', input);
    if (out.status === 'ok') await refresh();
    return out;
  }

  async function readPersona(kind: PersonaKind): Promise<Outcome<PersonaDocument> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const out = await call<PersonaDocument>(sid, 'persona.read', { kind });
    if (fresh(e, sid) && out.status === 'ok') {
      personaDocs.value = { ...personaDocs.value, [kind]: out.value };
      if (!(kind in drafts)) drafts[kind] = out.value.content;
    }
    return out;
  }

  function setDraft(kind: PersonaKind, content: string): void {
    drafts[kind] = content;
  }

  async function savePersona(
    kind: PersonaKind,
    reason?: string,
  ): Promise<Outcome<PersonaWriteOutcome> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const doc = personaDocs.value[kind];
    const e = epoch;
    const input: Record<string, unknown> = {
      kind,
      content: drafts[kind] ?? doc?.content ?? '',
      base_revision: doc?.revision ?? 0,
    };
    if (reason) input.reason = reason;
    const out = await call<PersonaWriteOutcome>(sid, 'persona.save', input);
    if (!fresh(e, sid)) return null;
    if (out.status === 'ok') {
      personaDocs.value = { ...personaDocs.value, [kind]: out.value.document };
      drafts[kind] = out.value.document.content;
      const errs = { ...personaErrors.value };
      delete errs[kind];
      personaErrors.value = errs;
      const unk = { ...unknownSaves.value };
      delete unk[kind];
      unknownSaves.value = unk;
    } else if (out.status === 'unknown') {
      // Ambiguous write: keep draft, surface, reconcile via explicit read.
      unknownSaves.value = { ...unknownSaves.value, [kind]: out.error };
    } else {
      personaErrors.value = { ...personaErrors.value, [kind]: out.error };
      // Read the authoritative current document so the UI can compare.
      void readPersona(kind);
    }
    return out;
  }

  /** Explicit reconciliation after an unknown save: reads current authority. */
  async function reconcilePersona(
    kind: PersonaKind,
  ): Promise<Outcome<PersonaDocument> | null> {
    const out = await readPersona(kind);
    if (out?.status === 'ok') {
      const unk = { ...unknownSaves.value };
      delete unk[kind];
      unknownSaves.value = unk;
    }
    return out;
  }

  async function listReviews(opts: {
    kind?: PersonaKind;
    state?: string;
    limit?: number;
    cursor?: string;
  } = {}): Promise<Outcome<PersonaReviewPage> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const input: Record<string, unknown> = {};
    if (opts.kind) input.kind = opts.kind;
    if (opts.state) input.state = opts.state;
    if (opts.limit !== undefined) input.limit = opts.limit;
    if (opts.cursor) input.cursor = opts.cursor;
    const out = await call<PersonaReviewPage>(sid, 'persona.reviews.list', input);
    if (fresh(e, sid) && out.status === 'ok') {
      reviews.value = opts.cursor
        ? [...reviews.value, ...(out.value.items ?? [])]
        : (out.value.items ?? []);
      reviewsCursor.value = out.value.next_cursor;
    }
    return out;
  }

  async function decideReview(
    reviewId: string,
    decision: 'accept' | 'reject',
  ): Promise<Outcome<unknown> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const out = await call<unknown>(sid, 'persona.review.decide', {
      review_id: reviewId,
      decision,
    });
    if (fresh(e, sid) && out.status === 'ok') {
      await listReviews();
      await refresh();
    }
    return out;
  }

  // ---- frozen ----

  async function readFrozen(): Promise<Outcome<FrozenReadback> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const out = await call<FrozenReadback>(sid, 'frozen.read', {});
    if (fresh(e, sid) && out.status === 'ok') frozen.value = out.value;
    return out;
  }

  // ---- actmem ----

  async function readActmem(
    sections: string[],
    maxChars: number,
  ): Promise<Outcome<ActivityResult> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const out = await call<ActivityResult>(sid, 'actmem.read', {
      sections,
      max_chars: maxChars,
    });
    if (!fresh(e, sid)) return null;
    if (out.status === 'ok') {
      actmem.value = out.value;
      actmemError.value = null;
    } else {
      actmemError.value = out.error;
    }
    return out;
  }

  async function patchWork(patch: WorkPatch): Promise<Outcome<ActivityResult> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const out = await call<ActivityResult>(sid, 'actmem.work.patch', { patch });
    if (!fresh(e, sid)) return null;
    if (out.status === 'ok') {
      actmem.value = out.value;
      actmemError.value = null;
    } else {
      actmemError.value = out.error;
    }
    return out;
  }

  async function ownerRead(): Promise<Outcome<OwnerActmemDocument> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const out = await call<OwnerActmemDocument>(sid, 'actmem.owner.read', {});
    if (!fresh(e, sid)) return null;
    if (out.status === 'ok') {
      ownerDoc.value = out.value;
      actmemError.value = null;
    } else {
      // Recovery payload (current doc) is allowed on failed outcomes.
      if (out.value) ownerDoc.value = out.value;
      actmemError.value = out.error;
    }
    return out;
  }

  async function ownerSave(
    markdown: string,
    baseRevision: number,
  ): Promise<Outcome<OwnerActmemDocument> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const out = await call<OwnerActmemDocument>(sid, 'actmem.owner.save', {
      markdown,
      base_revision: baseRevision,
    });
    if (!fresh(e, sid)) return null;
    if (out.status === 'ok') {
      ownerDoc.value = out.value;
      actmemError.value = null;
    } else {
      if (out.value) ownerDoc.value = out.value;
      actmemError.value = out.error;
    }
    return out;
  }

  // ---- memory ----

  async function searchMemory(opts: {
    query: string;
    collection?: string;
    cursor?: string;
    limit: number;
    budgetChars: number;
  }): Promise<Outcome<MemoryCardPage> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const input: Record<string, unknown> = {
      query: opts.query,
      limit: opts.limit,
      budget_chars: opts.budgetChars,
    };
    if (opts.collection) input.collection = opts.collection;
    if (opts.cursor) input.cursor = opts.cursor;
    const out = await call<MemoryCardPage>(sid, 'memory.search', input);
    if (!fresh(e, sid)) return null;
    if (out.status === 'ok') {
      memoryCards.value = out.value;
      memoryError.value = null;
    } else {
      memoryError.value = out.error;
    }
    return out;
  }

  async function expandMemory(
    cardId: string,
    expectedRevision: number,
    budgetChars: number,
  ): Promise<Outcome<MemoryEvidencePage> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const out = await call<MemoryEvidencePage>(sid, 'memory.expand', {
      card_id: cardId,
      expected_revision: expectedRevision,
      budget_chars: budgetChars,
    });
    if (!fresh(e, sid)) return null;
    if (out.status === 'ok') {
      evidence.value = out.value;
      memoryError.value = null;
    } else {
      memoryError.value = out.error;
    }
    return out;
  }

  async function mutateMemory(
    mutation: Record<string, unknown>,
  ): Promise<Outcome<MutationReceipt> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const out = await call<MutationReceipt>(sid, 'memory.mutate', { mutation });
    if (!fresh(e, sid)) return null;
    if (out.status === 'ok') {
      lastMutation.value = out.value;
      if (out.value.operation_id) {
        receipts.value = { ...receipts.value, [out.value.operation_id]: out.value };
      }
      memoryError.value = null;
    } else {
      memoryError.value = out.error;
      if (out.status === 'failed' && out.value) lastMutation.value = out.value;
    }
    return out;
  }

  async function receipt(operationId: string): Promise<Outcome<MutationReceipt> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const out = await call<MutationReceipt>(sid, 'memory.receipt', {
      operation_id: operationId,
    });
    if (fresh(e, sid) && out.status === 'ok') {
      receipts.value = { ...receipts.value, [operationId]: out.value };
    }
    return out;
  }

  // ---- evolution / cognition ----

  async function getPolicy(): Promise<Outcome<CognitivePolicy> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const out = await call<CognitivePolicy>(sid, 'policy.get', {});
    if (!fresh(e, sid)) return null;
    if (out.status === 'ok') {
      policy.value = out.value;
      policyError.value = null;
    } else {
      policyError.value = out.error;
    }
    return out;
  }

  async function setPolicy(opts: {
    enabled: boolean;
    minIntervalMs: number;
    baseRevision: number;
  }): Promise<Outcome<CognitivePolicy> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const out = await call<CognitivePolicy>(sid, 'policy.set', {
      enabled: opts.enabled,
      min_interval_ms: opts.minIntervalMs,
      base_revision: opts.baseRevision,
    });
    if (!fresh(e, sid)) return null;
    if (out.status === 'ok') {
      policy.value = out.value;
      policyError.value = null;
      await refresh();
    } else {
      policyError.value = out.error;
    }
    return out;
  }

  async function triggerCognition(): Promise<Outcome<TriggerResult> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const out = await call<TriggerResult>(sid, 'trigger', {});
    if (fresh(e, sid) && out.status === 'ok') {
      lastTrigger.value = out.value;
      await refresh();
    }
    return out;
  }

  async function cancelCognition(runId: string): Promise<Outcome<unknown> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const out = await call<unknown>(sid, 'cancel', { run_id: runId });
    if (fresh(e, sid) && out.status === 'ok') await refresh();
    return out;
  }

  async function listResults(opts: {
    cursor?: string;
    limit?: number;
  } = {}): Promise<Outcome<ResultsPage> | null> {
    const sid = sessionId.value;
    if (!sid) return null;
    const e = epoch;
    const input: Record<string, unknown> = {};
    if (opts.cursor) input.cursor = opts.cursor;
    if (opts.limit !== undefined) input.limit = opts.limit;
    const out = await call<ResultsPage>(sid, 'results.list', input);
    if (fresh(e, sid) && out.status === 'ok') results.value = out.value;
    return out;
  }

  return {
    sessionId,
    loading,
    status,
    blockReason,
    statusError,
    capabilityBlocked,
    needsSetup,
    frozenRecoveryRequired,
    personaDocs,
    drafts,
    personaErrors,
    unknownSaves,
    reviews,
    reviewsCursor,
    frozen,
    actmem,
    ownerDoc,
    actmemError,
    memoryCards,
    evidence,
    memoryError,
    lastMutation,
    receipts,
    policy,
    policyError,
    lastTrigger,
    results,
    currentRevisionOf,
    bind,
    unbind,
    refresh,
    initialize,
    readPersona,
    setDraft,
    savePersona,
    reconcilePersona,
    listReviews,
    decideReview,
    readFrozen,
    readActmem,
    patchWork,
    ownerRead,
    ownerSave,
    searchMemory,
    expandMemory,
    mutateMemory,
    receipt,
    getPolicy,
    setPolicy,
    triggerCognition,
    cancelCognition,
    listResults,
  };
}

export type VivyCognitiveController = ReturnType<typeof createVivyCognitiveController>;

let instance: VivyCognitiveController | null = null;

export function vivyCognitive(): VivyCognitiveController {
  if (!instance) instance = createVivyCognitiveController({ client: cognitiveClient });
  return instance;
}
