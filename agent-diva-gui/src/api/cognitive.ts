/**
 * DN-4D — typed client for the sealed `vivy/diva-cognitive` module.
 *
 * Sole backend path: `module.action.invoke` over the existing VivyClient
 * (`vivy_call` transport). These are host-side control actions, NOT desktop
 * native commands — desktop-host.ts has no role here.
 *
 * Contract facts (backend-separation-contracts.md C2-3 + captured fixture
 * docs/plans/diva-next/fixtures/closure-cognitive-actions.json):
 * - Action IDs are `diva.cognitive.<suffix>`; 20 closed suffixes.
 * - Every input carries `session_id` (correlation only, never authority).
 * - Results use the business envelope:
 *     { status: 'ok', value }
 *     { status: 'unavailable'|'failed'|'unknown',
 *       error: { code, message, retryable }, value? }
 *   `value` on a non-ok outcome is a safe recovery payload only.
 * - Writes are sent with { mutation: true } so the transport tags timeouts as
 *   unknownOutcome; we surface that as { status: 'unknown' } and NEVER replay.
 * - Input numbers must be safe integers (revisions, limits, budgets, ms).
 */
import { vivyClient } from './vivy/instance';
import type { VivyClient } from './vivy/client';
import { VivyCallError } from './vivy/contracts';

export const COGNITIVE_MODULE_ID = 'vivy/diva-cognitive';
export const COGNITIVE_ACTION_PREFIX = 'diva.cognitive.';

export type CognitiveAction =
  | 'status'
  | 'persona.initialize'
  | 'persona.read'
  | 'persona.save'
  | 'persona.reviews.list'
  | 'persona.review.decide'
  | 'frozen.read'
  | 'actmem.read'
  | 'actmem.work.patch'
  | 'actmem.owner.read'
  | 'actmem.owner.save'
  | 'memory.search'
  | 'memory.expand'
  | 'memory.mutate'
  | 'memory.receipt'
  | 'policy.get'
  | 'policy.set'
  | 'trigger'
  | 'cancel'
  | 'results.list';

const ALL_ACTIONS: ReadonlySet<string> = new Set<CognitiveAction>([
  'status',
  'persona.initialize',
  'persona.read',
  'persona.save',
  'persona.reviews.list',
  'persona.review.decide',
  'frozen.read',
  'actmem.read',
  'actmem.work.patch',
  'actmem.owner.read',
  'actmem.owner.save',
  'memory.search',
  'memory.expand',
  'memory.mutate',
  'memory.receipt',
  'policy.get',
  'policy.set',
  'trigger',
  'cancel',
  'results.list',
]);

/** Writes / side-effecting actions. A transport timeout on these is an
 *  unknown business outcome — surfaced, never silently retried. */
const MUTATION_ACTIONS: ReadonlySet<string> = new Set<CognitiveAction>([
  'persona.initialize',
  'persona.save',
  'persona.review.decide',
  'actmem.work.patch',
  'actmem.owner.save',
  'memory.mutate',
  'policy.set',
  'trigger',
  'cancel',
]);

export interface CognitiveErrorBody {
  code: string;
  message: string;
  retryable: boolean;
}

export type CognitiveOutcome<T> =
  | { status: 'ok'; value: T }
  | {
      status: 'unavailable' | 'failed' | 'unknown';
      error: CognitiveErrorBody;
      /** Optional safe recovery payload (e.g. current actmem revision). */
      value?: T;
    };

export type CognitiveOutcomeStatus = CognitiveOutcome<unknown>['status'];

/** Envelope/input shape violation — distinct from business failures. */
export class CognitiveProtocolError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'CognitiveProtocolError';
  }
}

// ---------------------------------------------------------------------------
// DTOs (domain aliases — wire shapes owned by laputa/garden/mentle, not us)
// ---------------------------------------------------------------------------

export type PersonaKind =
  | 'identity'
  | 'relationship'
  | 'redline'
  | 'user'
  | 'dream'
  | 'dark'
  | 'world'
  | 'mission';

export const PERSONA_KINDS: readonly PersonaKind[] = [
  'identity',
  'relationship',
  'redline',
  'user',
  'world',
  'dream',
  'dark',
  'mission',
];

/** The five files required for initialization (MISSION excluded). */
export const REQUIRED_PERSONA_KINDS: readonly PersonaKind[] = [
  'identity',
  'relationship',
  'redline',
  'user',
  'world',
];

export interface PersonaDocument {
  kind: PersonaKind;
  file_name: string;
  exists: boolean;
  valid: boolean;
  reason?: string | null;
  content: string;
  revision: number;
  content_hash: string;
  updated_at: string | null;
  pending_count: number;
}

export interface PersonaInitialization {
  identity: string;
  relationship: string;
  redline: string;
  user: string;
  world: string;
}

export interface PersonaWriteOutcome {
  document: PersonaDocument;
  changed: boolean;
}

export interface PersonaReview {
  id: string;
  kind: PersonaKind;
  base_revision: number;
  base_hash: string;
  proposed_markdown: string;
  actor: string;
  reason: string;
  created_at: string;
  state: 'pending' | 'accepted' | 'rejected' | 'stale';
  decided_at?: string | null;
}

export interface PersonaReviewPage {
  items: PersonaReview[];
  next_cursor?: string;
}

export type PersonaState = 'uninitialized' | 'incomplete' | 'ready';
export type FrozenState = 'not_captured' | 'ready' | 'recovery_required';
export type CognitionPhase =
  | 'disabled'
  | 'idle'
  | 'running'
  | 'paused'
  | 'recovery_required';

export interface CapabilityStatus {
  profile_id: string;
  scope?: Record<string, unknown>;
  destination_id: string;
  persona: {
    state: PersonaState;
    current_revisions: Record<string, number>;
  };
  frozen: { state: FrozenState; revisions?: Record<string, number> };
  memory: {
    backend_id: string;
    health: 'available' | 'degraded' | 'unavailable';
    reason_code: string;
    canonical_state: string;
    index_state: string;
  };
  cognition: {
    enabled: boolean;
    policy_revision: number;
    min_interval_ms: number;
    eligibility?: Record<string, unknown> | null;
    active_run_id: string;
    source_id: string;
    watermark: number;
    pending_through: number;
    phase: CognitionPhase;
    block_reason: string;
  };
}

/** frozen.read value — pass-through of the domain FrozenCore projection. */
export interface FrozenReadback {
  state: FrozenState;
  revisions?: Record<string, number>;
  [key: string]: unknown;
}

export interface ActivityResult {
  changed: boolean;
  revision: number;
  entries?: unknown[] | null;
}

export interface OwnerActmemDocument {
  revision: number;
  updated_at: string;
  pulse: string;
  recap: string;
  work: string;
  markdown: string;
  unclassified: boolean;
}

export interface WorkPatch {
  base_revision: number;
  changes: unknown[];
}

export interface MemoryCardPage {
  items: Array<Record<string, unknown>> | null;
  next_cursor: string;
}

export interface MemoryEvidencePage {
  items: Array<Record<string, unknown>> | null;
  truncated: boolean;
}

export interface MutationReceipt {
  operation_id: string;
  payload_digest: string;
  status: string;
  target_ref: string;
  revision: number;
  error_code: string;
  canonical_status: string;
  index_status: string;
}

export interface CognitivePolicy {
  enabled: boolean;
  min_interval_ms: number;
  policy_revision: number;
}

export interface TriggerResult {
  eligibility?: Record<string, unknown> | null;
  active_run_id?: string;
  [key: string]: unknown;
}

export interface ResultsPage {
  items: Array<Record<string, unknown>>;
  next_cursor?: string;
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

export interface CognitiveClient {
  call<T>(
    action: CognitiveAction,
    input: { session_id: string } & Record<string, unknown>,
  ): Promise<CognitiveOutcome<T>>;
}

function validateInput(input: Record<string, unknown>, action: string): void {
  const sid = input.session_id;
  if (typeof sid !== 'string' || sid.length === 0) {
    throw new CognitiveProtocolError(`${action}: session_id must be a non-empty string`);
  }
  const walk = (value: unknown, path: string): void => {
    if (typeof value === 'number') {
      if (!Number.isSafeInteger(value)) {
        throw new CognitiveProtocolError(
          `${action}: ${path} must be a safe integer (got ${String(value)})`,
        );
      }
      return;
    }
    if (Array.isArray(value)) {
      value.forEach((v, i) => walk(v, `${path}[${i}]`));
      return;
    }
    if (value !== null && typeof value === 'object') {
      for (const [k, v] of Object.entries(value)) walk(v, `${path}.${k}`);
    }
  };
  walk(input, 'input');
}

function parseOutcome<T>(raw: unknown, action: string): CognitiveOutcome<T> {
  if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) {
    throw new CognitiveProtocolError(`${action}: malformed outcome envelope`);
  }
  const env = raw as { status?: unknown; value?: unknown; error?: unknown };
  if (env.status === 'ok') {
    if (!('value' in env)) {
      throw new CognitiveProtocolError(`${action}: ok outcome missing value`);
    }
    return { status: 'ok', value: env.value as T };
  }
  if (env.status === 'unavailable' || env.status === 'failed' || env.status === 'unknown') {
    const err = env.error as CognitiveErrorBody | undefined;
    if (
      !err ||
      typeof err !== 'object' ||
      typeof err.code !== 'string' ||
      typeof err.message !== 'string' ||
      typeof err.retryable !== 'boolean'
    ) {
      throw new CognitiveProtocolError(`${action}: ${env.status} outcome missing error body`);
    }
    const out: CognitiveOutcome<T> = {
      status: env.status,
      error: { code: err.code, message: err.message, retryable: err.retryable },
    };
    if ('value' in env) out.value = env.value as T;
    return out;
  }
  throw new CognitiveProtocolError(`${action}: unknown outcome status ${String(env.status)}`);
}

export function createCognitiveClient(client: VivyClient): CognitiveClient {
  return {
    async call<T>(
      action: CognitiveAction,
      input: { session_id: string } & Record<string, unknown>,
    ): Promise<CognitiveOutcome<T>> {
      if (!ALL_ACTIONS.has(action)) {
        throw new CognitiveProtocolError(`unknown cognitive action: ${String(action)}`);
      }
      validateInput(input, action);
      try {
        const raw = await client.call<unknown>(
          'module.action.invoke',
          {
            module_id: COGNITIVE_MODULE_ID,
            action_id: `${COGNITIVE_ACTION_PREFIX}${action}`,
            input,
          },
          { mutation: MUTATION_ACTIONS.has(action) },
        );
        return parseOutcome<T>(raw, action);
      } catch (e) {
        if (e instanceof VivyCallError && e.unknownOutcome) {
          // Ambiguous write: surface unknown, never auto-replay.
          return {
            status: 'unknown',
            error: { code: 'outcome_unknown', message: e.message, retryable: false },
          };
        }
        throw e;
      }
    },
  };
}

export const cognitiveClient = createCognitiveClient(vivyClient);
