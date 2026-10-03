/**
 * DN-4 masks slice — VIVY mask catalog/selection adapter.
 *
 * Sole backend path: `module.action.invoke` on the compiled `vivy/masks`
 * module (action IDs verified in the sealed Generation manifest):
 *   vivy.masks.catalog.list|get|create|update|delete
 *   vivy.masks.selection.get|set
 *
 * Contract facts (internal/modules/masks/actions.go on the pinned VIVY):
 * - create requires a client-generated `operation_id` UUID for idempotency.
 * - update/delete/select carry `expected_revision` — stale values fail with
 *   error.data.code = "revision_conflict" (+ data.current_revision).
 * - delete of a bound mask fails with data.code = "mask_in_use"
 *   (+ data.reference_count).
 * - built_in definitions are read-only.
 * - selection is per session_id.
 */
import { vivyClient } from './vivy/instance'

const MODULE_ID = 'vivy/masks'

export interface MaskDefinition {
  id: string
  name: string
  description: string
  body: string
  revision: number
  digest: string
  built_in: boolean
  generation_id: string
}

export interface MaskPage {
  items: MaskDefinition[]
  next_after_id: string
}

export interface MaskSelection {
  session_id: string
  mask_id: string
  revision: number
  available: boolean
  inactive_reason: '' | 'not_compiled'
}

/** Structured error surface for mask actions. `code` mirrors the wire
 *  data.code when the backend tags it (e.g. "revision_conflict",
 *  "mask_in_use", "not_found"); `currentRevision` lets callers resync. */
export class MaskActionError extends Error {
  constructor(
    message: string,
    readonly code: string | null,
    readonly currentRevision?: number,
    readonly referenceCount?: number,
  ) {
    super(message)
    this.name = 'MaskActionError'
  }
}

interface ActionErrorData {
  code?: string
  current_revision?: number
  reference_count?: number
}

function unwrapError(e: unknown): never {
  const err = e as { kind?: string; code?: number; message?: string; data?: ActionErrorData | null }
  const data = err?.data ?? undefined
  throw new MaskActionError(
    err?.message ?? String(e),
    typeof data?.code === 'string' ? data.code : null,
    typeof data?.current_revision === 'number' ? data.current_revision : undefined,
    typeof data?.reference_count === 'number' ? data.reference_count : undefined,
  )
}

async function invoke<T>(actionId: string, input: unknown): Promise<T> {
  try {
    return await vivyClient.call<T>(
      'module.action.invoke',
      { module_id: MODULE_ID, action_id: actionId, input },
      { mutation: true },
    )
  } catch (e) {
    unwrapError(e)
  }
}

export async function listMasks(opts: { afterId?: string; limit?: number } = {}): Promise<MaskPage> {
  return invoke<MaskPage>('vivy.masks.catalog.list', {
    after_id: opts.afterId ?? '',
    limit: opts.limit ?? 0,
  })
}

export async function listAllMasks(limit = 100): Promise<MaskDefinition[]> {
  const out: MaskDefinition[] = []
  let after = ''
  for (;;) {
    const page = await listMasks({ afterId: after, limit })
    out.push(...page.items)
    if (!page.next_after_id || page.items.length === 0) return out
    after = page.next_after_id
  }
}

export async function getMask(id: string): Promise<MaskDefinition> {
  return invoke<MaskDefinition>('vivy.masks.catalog.get', { id })
}

export async function createMask(input: {
  name: string
  description: string
  body: string
}): Promise<MaskDefinition> {
  return invoke<MaskDefinition>('vivy.masks.catalog.create', {
    operation_id: crypto.randomUUID(),
    name: input.name,
    description: input.description,
    body: input.body,
  })
}

export async function updateMask(input: {
  id: string
  expectedRevision: number
  name: string
  description: string
  body: string
}): Promise<MaskDefinition> {
  return invoke<MaskDefinition>('vivy.masks.catalog.update', {
    id: input.id,
    expected_revision: input.expectedRevision,
    name: input.name,
    description: input.description,
    body: input.body,
  })
}

export async function deleteMask(id: string, expectedRevision: number): Promise<{ deleted: boolean }> {
  return invoke<{ deleted: boolean }>('vivy.masks.catalog.delete', {
    id,
    expected_revision: expectedRevision,
  })
}

export async function getMaskSelection(sessionId: string): Promise<MaskSelection> {
  return invoke<MaskSelection>('vivy.masks.selection.get', { session_id: sessionId })
}

export async function setMaskSelection(
  sessionId: string,
  maskId: string,
  expectedRevision: number,
): Promise<MaskSelection> {
  return invoke<MaskSelection>('vivy.masks.selection.set', {
    session_id: sessionId,
    mask_id: maskId,
    expected_revision: expectedRevision,
  })
}
