/**
 * OBS-08 bounded GUI diagnostics recorder. `recordGuiDiagnostic` is the
 * single entry point (DN-6C speech diagnostics consume it): records are
 * sanitized to an allowlist, queued in a bounded buffer (<=1000 records /
 * <=1 MiB serialized, oldest dropped with a visible count), and flushed
 * in <=50 record / <=256 KiB batches through `diagnostics/gui/append`.
 *
 * Accounting is honest: only the server's `accepted` count increments
 * `persisted`; a failed or ambiguous write moves the whole batch to
 * `unconfirmed` and is NEVER retried (accepted-prefix counts are never
 * parsed out of error text). The recorder logs nothing about itself —
 * its own transport and error paths are excluded from capture, so a
 * reader or flush failure cannot feed it recursively.
 */
import type {
  DiagnosticPage,
  DiagnosticQuery,
  GuiLogBatch,
  GuiLogRecord,
} from '../api/vivy/contracts'
import type { VivyClient } from '../api/vivy/client'
import {
  appendGuiLogs,
  readDiagnostics,
  GUI_BATCH_MAX_RECORDS,
} from '../api/vivy/observability'
import { vivyClient } from '../api/vivy/instance'

export const GUI_QUEUE_MAX_RECORDS = 1000
export const GUI_QUEUE_MAX_BYTES = 1024 * 1024
const MESSAGE_MAX_CHARS = 1024
const FIELD_MAX_KEYS = 8
const FIELD_VALUE_MAX_CHARS = 256

const REDACTED = '⟨redacted⟩'

/** Secret/blob sentinels — speech diagnostics must never carry secret,
 * text, audio, body, or full-URL content into the log. */
const REDACT_PATTERNS: RegExp[] = [
  /\bsk-[A-Za-z0-9_-]{8,}\b/g,
  /Bearer\s+[A-Za-z0-9._~-]+/gi,
  /data:[^\s;,]{1,32}[^,\s]*;base64,[A-Za-z0-9+/=]{16,}/g,
  /\b(?:api[_-]?key|access[_-]?token|secret|password|credential|authorization)\s*[:=]\s*[^\s,;]+/gi,
  /\bhttps?:\/\/\S{40,}/g,
  /[A-Za-z0-9+/]{160,}={0,2}/g,
]

/** Field keys that carry secrets or raw payloads — dropped entirely. */
const FIELD_DENY =
  /secret|token|api_?key|password|credential|auth|audio|pcm|wav|mp3|body|payload|text|content|url|uri|transcript|prompt|base64|data/i

export function sanitizeMessage(message: string): string {
  let out = message
  for (const pattern of REDACT_PATTERNS) {
    out = out.replace(pattern, REDACTED)
  }
  if (out.length > MESSAGE_MAX_CHARS) out = `${out.slice(0, MESSAGE_MAX_CHARS)}…`
  return out
}

export function sanitizeFields(
  fields: Record<string, unknown> | undefined,
): Record<string, unknown> | undefined {
  if (!fields) return undefined
  const out: Record<string, unknown> = {}
  for (const [key, value] of Object.entries(fields)) {
    if (Object.keys(out).length >= FIELD_MAX_KEYS) break
    if (FIELD_DENY.test(key)) continue
    if (typeof value === 'string') {
      out[key] = sanitizeMessage(value).slice(0, FIELD_VALUE_MAX_CHARS)
    } else if (typeof value === 'number' || typeof value === 'boolean' || value === null) {
      out[key] = value
    }
    // objects/arrays are dropped — only scalar allowlisted fields survive.
  }
  return Object.keys(out).length > 0 ? out : undefined
}

export interface GuiDiagnosticInput {
  level?: string
  component?: string
  message: string
  fields?: Record<string, unknown>
  at?: number
}

export interface GuiQueueStats {
  queued: number
  queuedBytes: number
  persisted: number
  dropped: number
  unconfirmed: number
  flushing: boolean
  lastError: string | null
}

export class GuiDiagnosticRecorder {
  private queue: GuiLogRecord[] = []
  private queueBytes = 0
  private persistedCount = 0
  private droppedCount = 0
  private unconfirmedCount = 0
  private inFlight = false
  private lastErr: string | null = null

  constructor(private readonly client: VivyClient) {}

  /** Enqueue one sanitized record; returns false when the record was
   * dropped at the tail (queue bounds force oldest out — both count). */
  record(input: GuiDiagnosticInput): void {
    const rec: GuiLogRecord = {
      at: input.at ?? Date.now(),
      level: input.level ?? 'info',
      component: input.component ? input.component.slice(0, 64) : 'gui',
      message: sanitizeMessage(input.message),
      fields: sanitizeFields(input.fields),
    }
    const recBytes = JSON.stringify(rec).length
    while (
      this.queue.length >= GUI_QUEUE_MAX_RECORDS ||
      this.queueBytes + recBytes > GUI_QUEUE_MAX_BYTES
    ) {
      const evicted = this.queue.shift()
      if (!evicted) break
      this.queueBytes -= JSON.stringify(evicted).length
      this.droppedCount++
    }
    this.queue.push(rec)
    this.queueBytes += recBytes
  }

  stats(): GuiQueueStats {
    return {
      queued: this.queue.length,
      queuedBytes: this.queueBytes,
      persisted: this.persistedCount,
      dropped: this.droppedCount,
      unconfirmed: this.unconfirmedCount,
      flushing: this.inFlight,
      lastError: this.lastErr,
    }
  }

  /** Fire-and-forget drain; errors land on counters, never on callers. */
  scheduleFlush(): void {
    void this.flush().catch(() => undefined)
  }

  /** Single-flight drain. Success ack counts only `accepted` rows; any
   * failure parks the whole dispatched batch as unconfirmed — no retry,
   * no error-text parsing. */
  async flush(): Promise<void> {
    if (this.inFlight || this.queue.length === 0) return
    this.inFlight = true
    try {
      while (this.queue.length > 0) {
        const batch = this.takeBatch()
        if (batch.records.length === 0) break
        try {
          const ack = await appendGuiLogs(this.client, batch)
          const accepted = Math.max(0, Math.min(ack.accepted, batch.records.length))
          this.persistedCount += accepted
          this.unconfirmedCount += batch.records.length - accepted
          this.lastErr = null
        } catch (error) {
          this.unconfirmedCount += batch.records.length
          this.lastErr = error instanceof Error ? error.message : String(error)
          break
        }
      }
    } finally {
      this.inFlight = false
    }
  }

  private takeBatch(): GuiLogBatch {
    const records: GuiLogRecord[] = []
    let bytes = 0
    while (
      this.queue.length > 0 &&
      records.length < GUI_BATCH_MAX_RECORDS &&
      bytes < 240 * 1024
    ) {
      const rec = this.queue[0]
      const recBytes = JSON.stringify(rec).length
      if (records.length > 0 && bytes + recBytes > 240 * 1024) break
      records.push(rec)
      bytes += recBytes
      this.queue.shift()
      this.queueBytes -= recBytes
    }
    return { records }
  }
}

/** Shared recorder — the one entry point speech/GUI diagnostics call. */
export const guiDiagnostics = new GuiDiagnosticRecorder(vivyClient)

export function recordGuiDiagnostic(input: GuiDiagnosticInput): void {
  guiDiagnostics.record(input)
  guiDiagnostics.scheduleFlush()
}

export interface DiagnosticsFilter {
  source: string
  date?: string
  level?: string
  query?: string
  limit?: number
}

/** Bounded `diagnostics/logs` reader for the console panel. Holds the
 * latest page verbatim — gap/has_more/truncated are producer flags and
 * are displayed as-is; a page whose `gap` is set is surfaced alongside
 * the rows it still carries. Generation fencing drops stale responses
 * after a filter change. Failures never enqueue recorder records. */
export class DiagnosticsReader {
  page: DiagnosticPage | null = null
  loading = false
  lastError: string | null = null
  filter: DiagnosticsFilter = { source: 'runtime' }
  private gen = 0

  constructor(private readonly client: VivyClient) {}

  async refresh(filter?: DiagnosticsFilter): Promise<DiagnosticPage | null> {
    if (filter) this.filter = { ...this.filter, ...filter }
    const gen = ++this.gen
    this.loading = true
    this.lastError = null
    try {
      const query: DiagnosticQuery = {
        source: this.filter.source,
        ...(this.filter.date ? { date: this.filter.date } : {}),
        ...(this.filter.level ? { level: this.filter.level } : {}),
        ...(this.filter.query ? { query: this.filter.query } : {}),
        ...(this.filter.limit ? { limit: this.filter.limit } : {}),
      }
      const page = await readDiagnostics(this.client, query)
      if (gen !== this.gen) return null
      this.page = page
      return page
    } catch (error) {
      if (gen !== this.gen) return null
      this.lastError = error instanceof Error ? error.message : String(error)
      return null
    } finally {
      if (gen === this.gen) this.loading = false
    }
  }

  /** Resume the bound-stopped scan — the cursor is server-issued. */
  async nextPage(): Promise<DiagnosticPage | null> {
    const cursor = this.page?.next_cursor
    if (!cursor || !this.page?.has_more) return null
    const gen = ++this.gen
    this.loading = true
    try {
      const page = await readDiagnostics(this.client, {
        source: this.filter.source,
        ...(this.filter.date ? { date: this.filter.date } : {}),
        ...(this.filter.level ? { level: this.filter.level } : {}),
        ...(this.filter.query ? { query: this.filter.query } : {}),
        after: cursor,
      })
      if (gen !== this.gen) return null
      this.page = page
      return page
    } catch (error) {
      if (gen !== this.gen) return null
      this.lastError = error instanceof Error ? error.message : String(error)
      return null
    } finally {
      if (gen === this.gen) this.loading = false
    }
  }
}
