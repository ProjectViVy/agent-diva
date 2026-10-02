/**
 * GUI capability ledger (DN-1 revision). The old static `MANAGER`/`LOCAL`
 * labels are replaced by the DN-0 disposition per entry plus a negotiated
 * state resolved from the VIVY `initialize` capability set and transport
 * connectivity. This remains descriptive evidence — authority lives in the
 * VIVY backend; the ledger never substitutes a fake-success or a
 * localStorage authority.
 */

export type CapabilityDisposition =
  /** Verified VIVY RPC/action target (backend-separation-contracts §3). */
  | 'existing-backend'
  /** Shell-owned local op (window/tray/host files). */
  | 'native'
  /** No VIVY surface; mapped to a blocked Story (DN-3/4/6). */
  | 'backend-gap'
  /** Dead or removed entrypoint; retirement recorded pending owner call. */
  | 'retire-pending'

export type CapabilityState =
  /** Negotiated VIVY capability present, or a shell-owned native op. */
  | 'available'
  /** No negotiated surface exists for this entry (gap or retired). */
  | 'absent'
  /** A negotiated capability exists but the transport is down. */
  | 'disconnected'
  /** A live call through the negotiated capability failed. */
  | 'failed'

export interface GuiCapability {
  id: string
  entrypoint: string
  disposition: CapabilityDisposition
  /**
   * The `initialize` capability string this entry negotiates against.
   * Null for native entries and entries without a backend surface.
   */
  vivyCapability: string | null
  authority: string
  risk: 'read' | 'write' | 'execute' | 'lifecycle'
  failure: string
  verification: string
}

export const GUI_CAPABILITIES: readonly GuiCapability[] = [
  {
    id: 'chat.turn',
    entrypoint: 'turn/start, run/cancel',
    disposition: 'existing-backend',
    vivyCapability: 'turn',
    authority: 'VIVY runtime via turn/* + run/*',
    risk: 'execute',
    failure: 'Surface VIVY run/turn failure; never synthesize a completed turn.',
    verification: 'vivy-session projection tests + DN-2 live turn',
  },
  {
    id: 'session.lifecycle',
    entrypoint: 'session/list, session/get, session/messages, session/rename, session/delete',
    disposition: 'existing-backend',
    vivyCapability: 'session',
    authority: 'VIVY session journal',
    risk: 'write',
    failure: 'Keep the last projection and expose refresh/error state.',
    verification: 'vivy-session projection tests + native session smoke',
  },
  {
    id: 'plan.lifecycle',
    entrypoint: 'plan/get, plan/enter, plan/decide, plan/leave, session/work/*',
    disposition: 'existing-backend',
    vivyCapability: 'plan',
    authority: 'VIVY plan mode + work view (semantics differ from old plan approval; DN-3 design)',
    risk: 'execute',
    failure: 'Fail closed and reconcile the authoritative revision.',
    verification: 'DN-3 field mapping review',
  },
  {
    id: 'command.approval',
    entrypoint: 'approval/list, approval/respond',
    disposition: 'existing-backend',
    vivyCapability: 'approval',
    authority: 'VIVY approval coordinator',
    risk: 'execute',
    failure: 'Preserve typed not-found semantics and reconcile pending state.',
    verification: 'projection pending-interaction tests + DN-2 live approval',
  },
  {
    id: 'command.rules',
    entrypoint: 'commands/list, commands/expand',
    disposition: 'existing-backend',
    vivyCapability: 'commands.list',
    authority: 'VIVY commands surface',
    risk: 'read',
    failure: 'Expose list failure; do not mutate optimistic state as fact.',
    verification: 'DN-3 commands surface review',
  },
  {
    id: 'config.runtime',
    entrypoint: 'settings/get, settings/update',
    disposition: 'existing-backend',
    vivyCapability: 'settings.get',
    authority: 'VIVY settings journal',
    risk: 'write',
    failure: 'Expose validation or availability failure.',
    verification: 'DN-2 settings smoke',
  },
  {
    id: 'providers',
    entrypoint: 'settings/providers, settings/providers/upsert|delete|refresh, settings/model/select',
    disposition: 'existing-backend',
    vivyCapability: 'settings.providers',
    authority: 'VIVY provider catalog',
    risk: 'write',
    failure: 'Preserve provider error and never report a false successful test.',
    verification: 'DN-2 provider smoke',
  },
  {
    id: 'skills.files',
    entrypoint: 'skills/list, skills/get, skills/revisions',
    disposition: 'existing-backend',
    vivyCapability: 'skills.list',
    authority: 'VIVY skills service (enable/upload gap recorded in DN-0)',
    risk: 'write',
    failure: 'Expose failure and retain server projection.',
    verification: 'DN-3 skills surface review',
  },
  {
    id: 'mcp.servers',
    entrypoint: 'settings/mcp, settings/mcp/upsert|delete|probe',
    disposition: 'existing-backend',
    vivyCapability: 'settings.mcp',
    authority: 'VIVY MCP registry',
    risk: 'execute',
    failure: 'Expose connection failure; never infer connected state locally.',
    verification: 'DN-2/3 MCP surface review',
  },
  {
    id: 'cron.jobs',
    entrypoint: 'cron/list, cron/create, cron/update, cron/delete, cron/trigger, cron/stop',
    disposition: 'existing-backend',
    vivyCapability: 'cron.list',
    authority: 'VIVY cron service',
    risk: 'execute',
    failure: 'Expose scheduler failure and refresh server state.',
    verification: 'DN-3 cron surface review',
  },
  {
    id: 'memory.home',
    entrypoint: 'none — unresolved domain',
    disposition: 'backend-gap',
    vivyCapability: null,
    authority: 'No VIVY memory RPC surface; DN-4 design required',
    risk: 'write',
    failure: 'Report absent; never apply memory writes locally.',
    verification: 'Capability state resolves absent; DN-4 contract review',
  },
  {
    id: 'memory.laputa',
    entrypoint: 'none',
    disposition: 'retire-pending',
    vivyCapability: null,
    authority: 'Retired Laputa proposal and section governance',
    risk: 'write',
    failure: 'Surface is gone; no replacement until DN-4.',
    verification: 'Capability ledger test asserts retire classification',
  },
  {
    id: 'autodream',
    entrypoint: 'none — unresolved domain',
    disposition: 'backend-gap',
    vivyCapability: null,
    authority: 'Evolution domain shape-overlap only (generations/evals); DN-4',
    risk: 'execute',
    failure: 'Report absent; never claim a run was started.',
    verification: 'Capability state resolves absent; DN-4 contract review',
  },
  {
    id: 'todo.execution',
    entrypoint: 'session.todos, session.todo.update',
    disposition: 'existing-backend',
    vivyCapability: 'session.todos',
    authority: 'VIVY work/todos projection',
    risk: 'write',
    failure: 'Preserve missing/conflict semantics.',
    verification: 'DN-3 work surface review',
  },
  {
    id: 'audit.logs',
    entrypoint: 'none',
    disposition: 'backend-gap',
    vivyCapability: null,
    authority: 'No VIVY audit RPC (DN-0 §3.5)',
    risk: 'read',
    failure: 'Report absent; do not substitute an empty successful result.',
    verification: 'Capability state resolves absent',
  },
  {
    id: 'token.statistics',
    entrypoint: 'stats/tokens',
    disposition: 'existing-backend',
    vivyCapability: 'stats.tokens',
    authority: 'VIVY token ledger snapshot',
    risk: 'read',
    failure: 'Expose query failure and retain last known data.',
    verification: 'DN-2 stats smoke',
  },
  {
    id: 'mask.lifecycle',
    entrypoint: 'none — unresolved domain',
    disposition: 'backend-gap',
    vivyCapability: null,
    authority: 'maskcontract has no control RPC; DN-4 design required',
    risk: 'write',
    failure: 'Report absent; never switch mask locally.',
    verification: 'Capability state resolves absent; DN-4 contract review',
  },
  {
    id: 'gateway.process',
    entrypoint: 'none — embedded host IS the runtime',
    disposition: 'retire-pending',
    vivyCapability: null,
    authority: 'No gateway exists to manage (DN-0 §3.5)',
    risk: 'lifecycle',
    failure: 'Surface is gone; bridge init envelope carries identity instead.',
    verification: 'Capability ledger test asserts retire classification',
  },
  {
    id: 'desktop.service',
    entrypoint: 'none — embedded host IS the runtime',
    disposition: 'retire-pending',
    vivyCapability: null,
    authority: 'No service install surface in the thin shell (DN-0 §3.5)',
    risk: 'lifecycle',
    failure: 'Surface is gone; no service controller to call.',
    verification: 'Capability ledger test asserts retire classification',
  },
  {
    id: 'desktop.preferences',
    entrypoint: 'desktop-host prefs (shell-owned)',
    disposition: 'native',
    vivyCapability: null,
    authority: 'Tauri shell preference store',
    risk: 'write',
    failure: 'Keep current preferences and surface persistence error.',
    verification: 'Shell lifecycle tests',
  },
  {
    id: 'desktop.logs',
    entrypoint: 'desktop-host log append/read (shell-owned)',
    disposition: 'native',
    vivyCapability: null,
    authority: 'Tauri local log files',
    risk: 'write',
    failure: 'Report I/O failure without fabricating records.',
    verification: 'Shell lifecycle tests',
  },
  {
    id: 'desktop.pet',
    entrypoint: 'desktop-host pet window ops (shell-owned)',
    disposition: 'native',
    vivyCapability: null,
    authority: 'Tauri desktop pet host',
    risk: 'lifecycle',
    failure: 'Surface host/media failure without mutating backend state.',
    verification: 'DN-6 pet surface review',
  },
  {
    id: 'manager.direct-browser-http',
    entrypoint: 'none',
    disposition: 'retire-pending',
    vivyCapability: null,
    authority: 'No browser-direct Manager path exists',
    risk: 'execute',
    failure: 'Unavailable; GUI goes through the shell bridge only.',
    verification: 'Capability ledger test asserts retire classification',
  },
  {
    id: 'plan.history.overlay',
    entrypoint: 'none',
    disposition: 'retire-pending',
    vivyCapability: null,
    authority: 'Removed from the product surface',
    risk: 'read',
    failure: 'No entrypoint; callers must not restore the obsolete overlay.',
    verification: 'Capability ledger test asserts retire classification',
  },
] as const

/**
 * Resolve the negotiated state for every ledger entry.
 *
 * @param initCaps capability strings reported by VIVY `initialize`
 *   (null/undefined = no negotiated envelope seen yet).
 * @param connected  transport state: false marks every existing-backend
 *   entry `disconnected` regardless of the negotiated set.
 */
export function negotiateCapabilities(
  initCaps: readonly string[] | null | undefined,
  connected: boolean,
): Map<string, CapabilityState> {
  const negotiated = new Map<string, CapabilityState>()
  const caps = new Set(initCaps ?? [])
  for (const entry of GUI_CAPABILITIES) {
    switch (entry.disposition) {
      case 'native':
        negotiated.set(entry.id, 'available')
        break
      case 'backend-gap':
      case 'retire-pending':
        negotiated.set(entry.id, 'absent')
        break
      case 'existing-backend':
        if (!connected) negotiated.set(entry.id, 'disconnected')
        else if (entry.vivyCapability === null || !caps.has(entry.vivyCapability))
          negotiated.set(entry.id, 'absent')
        else negotiated.set(entry.id, 'available')
        break
    }
  }
  return negotiated
}

/** Mark an entry failed after a live call through it errored. */
export function markCapabilityFailed(
  negotiated: Map<string, CapabilityState>,
  id: string,
): Map<string, CapabilityState> {
  const next = new Map(negotiated)
  if (next.has(id)) next.set(id, 'failed')
  return next
}
