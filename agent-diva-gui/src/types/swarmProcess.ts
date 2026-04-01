/** 与 `ProcessEventV0` / `process-events-v0.md` 对齐的线格式（camelCase + `name` snake_case）。 */
export type SwarmRunStopReasonWire = 'done' | 'budgetExceeded' | 'timeout' | 'error';

export interface ProcessEventWire {
  schemaVersion: number;
  name: string;
  message: string;
  phaseId?: string;
  correlationId?: string;
  toolName?: string;
  stopReason?: SwarmRunStopReasonWire;
}

export function parseProcessEventWire(raw: unknown): ProcessEventWire | null {
  if (!raw || typeof raw !== 'object') return null;
  const o = raw as Record<string, unknown>;
  const schemaVersion = typeof o.schemaVersion === 'number' ? o.schemaVersion : Number(o.schemaVersion);
  const name = typeof o.name === 'string' ? o.name : '';
  const message = typeof o.message === 'string' ? o.message : '';
  if (!Number.isFinite(schemaVersion) || !name) return null;
  const ev: ProcessEventWire = { schemaVersion, name, message };
  if (typeof o.phaseId === 'string') ev.phaseId = o.phaseId;
  if (typeof o.correlationId === 'string') ev.correlationId = o.correlationId;
  if (typeof o.toolName === 'string') ev.toolName = o.toolName;
  if (
    o.stopReason === 'done' ||
    o.stopReason === 'budgetExceeded' ||
    o.stopReason === 'timeout' ||
    o.stopReason === 'error'
  ) {
    ev.stopReason = o.stopReason;
  }
  return ev;
}
