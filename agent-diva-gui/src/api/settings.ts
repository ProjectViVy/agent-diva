/**
 * DN-3 slice A — VIVY-backed provider/model settings adapter.
 *
 * Replaces the legacy whole-config load/save and provider DTO commands with
 * the VIVY settings surface: `settings/get`, `settings/providers`,
 * `settings/providers/upsert|delete|refresh`, `settings/model/select`.
 *
 * Credentials are write-only end to end: a typed key goes out once as
 * `api_key`; the wire only ever returns `api_key_set`. The backend owns the
 * vendor catalog — this module holds no provider list of its own.
 *
 * Selection rule (mirrors `settings.FindProvider`): a registry entry matches
 * by (normalized adapter bundle, base_url). `settings/model/select` accepts a
 * target only when it is the config default, a registry-entry model, or a
 * catalog-endpoint model — so this adapter always materializes a registry
 * entry for the selected provider before calling `settings/model/select`.
 */
import { vivyClient } from './vivy/instance'
import type {
  ProviderUpsertParams,
  VivyCatalogEntry,
  VivyProviderEntryResult,
  VivyProvidersResult,
  VivySettingsResult,
} from './vivy/contracts'
import type { ConfigStatusReport, ProviderStatusSummary } from './desktop'

export interface ProviderSpec {
  /** UI key: vendor name, `vendor#adapter` for multi-endpoint vendors, or the
   * registry entry id for providers with no catalog match. */
  name: string
  /** Sealed adapter family (openai-completions|openai-responses|anthropic-messages). */
  api_type: string
  source?: 'builtin' | 'custom' | string
  display_name: string
  default_model?: string | null
  default_api_base: string
  models: string[]
  custom_models: string[]
  /** False when the sealed adapter is DEFERRED-INDEFINITE in this Generation. */
  executable?: boolean
}

export interface ProviderConfigEntry {
  /** Always '' — credentials are redacted on the wire (`api_key_set` only). */
  apiKey: string
  apiBase: string
  source: 'providers' | 'custom_providers'
}

export interface ProviderModelCatalog {
  models: string[]
  custom_models: string[]
}

export interface ProviderModelTestResult {
  ok: boolean
  message: string
  latency_ms?: number
}

export interface CustomProviderPayload {
  /** Registry entry id for edits; omitted creates a `custom-*` id server-side. */
  id?: string
  displayName: string
  apiKey?: string
  apiBase: string
  defaultModel: string
  models?: string[]
}

export interface ProviderSettingsSnapshot {
  providers: ProviderSpec[]
  statusReport: ConfigStatusReport
  providerConfigs: Record<string, ProviderConfigEntry>
  runtime: {
    provider: string
    apiBase: string
    model: string
    /** True when a credential resolves for the active selection. */
    apiKeySet: boolean
  }
  entryByName: Record<string, VivyProviderEntryResult>
  /** api_key_set per provider name (spec.name) for UI "key configured" hints. */
  keySetByName: Record<string, boolean>
  readOnly: boolean
  frozen: boolean
}

/** Mirrors backend `legacyProviderAliases` (settings/provider_migration.go). */
const LEGACY_ADAPTER_ALIASES: Record<string, string> = {
  deepseek: 'openai-completions',
  openai: 'openai-completions',
  anthropic: 'anthropic-messages',
}

const normalizeAdapter = (value: string) => LEGACY_ADAPTER_ALIASES[value] ?? value

const endpointMatchesEntry = (
  entry: VivyProviderEntryResult,
  adapter: string,
  baseUrl: string,
) =>
  normalizeAdapter(entry.bundle) === normalizeAdapter(adapter) &&
  entry.base_url === baseUrl

function buildSpec(
  name: string,
  displayName: string,
  apiType: string,
  baseUrl: string,
  defaultModel: string | null,
  catalogModels: string[],
  entry: VivyProviderEntryResult | undefined,
  source: 'builtin' | 'custom',
  executable: boolean,
): ProviderSpec {
  const entryOnly = entry ? entry.models.filter((m) => !catalogModels.includes(m)) : []
  return {
    name,
    api_type: apiType,
    source,
    display_name: displayName,
    default_model: entry?.default_model || defaultModel,
    default_api_base: entry?.base_url || baseUrl,
    models: [...new Set([...catalogModels, ...(entry?.models ?? [])])],
    custom_models: entryOnly,
    executable,
  }
}

function specIsActive(
  spec: ProviderSpec,
  prov: VivyProvidersResult,
): boolean {
  const activeProvider = prov.active_provider || prov.config_provider
  if (!activeProvider) return false
  // A stored vendor name resolves to exactly that vendor's spec.
  if (spec.name === activeProvider) return true
  // Adapter equality plus base_url agreement picks the right sibling when
  // several vendors share an adapter family (e.g. openai-completions).
  if (normalizeAdapter(spec.api_type) !== normalizeAdapter(activeProvider)) return false
  if (prov.active_base_url !== '' && prov.active_base_url !== spec.default_api_base) return false
  // A bare adapter with an empty base_url cannot disambiguate vendors — only
  // legacy-alias vendor names carry meaning there.
  if (prov.active_base_url === '' && LEGACY_ADAPTER_ALIASES[activeProvider] === undefined) {
    return false
  }
  return true
}

function buildStatusReport(
  specs: ProviderSpec[],
  entryByName: Record<string, VivyProviderEntryResult>,
  prov: VivyProvidersResult,
  settings: VivySettingsResult,
): ConfigStatusReport {
  const activeProfile = (settings.provider_profiles ?? []).find(
    (p) => p.state === 'READY' || p.state === 'UNCONFIGURED',
  )
  const providers: ProviderStatusSummary[] = specs.map((spec) => {
    const entry = entryByName[spec.name]
    const current = specIsActive(spec, prov)
    const configured = !!entry?.api_key_set || (current && settings.api_key_set)
    const executable = spec.executable !== false
    return {
      name: spec.name,
      display_name: spec.display_name,
      default_model: spec.default_model,
      configurable: true,
      configured,
      ready: executable && configured,
      uses_api_base: true,
      provider_for_default_model: current,
      current,
      model: current ? prov.active_model : spec.default_model,
      api_base: entry?.base_url ?? spec.default_api_base,
      missing_fields: executable && !configured ? ['api_key'] : [],
    }
  })
  const activeConfigured = providers.some((p) => p.current && p.ready)
  const warnings: string[] = []
  if (prov.read_only || settings.read_only) warnings.push('settings read-only')
  if (prov.frozen || settings.frozen) warnings.push('provider selection frozen')
  return {
    config: {
      config_path: '',
      config_dir: '',
      runtime_dir: '',
      workspace: '',
      cron_store: '',
      bridge_dir: '',
      whatsapp_auth_dir: '',
      whatsapp_media_dir: '',
    },
    default_model: prov.active_model || prov.config_model,
    default_provider: prov.active_provider || prov.config_provider,
    logging: { level: '', format: '', dir: '' },
    providers,
    channels: [],
    cron_jobs: 0,
    mcp_servers: { configured: 0, disabled: 0 },
    doctor: {
      valid: true,
      ready: activeConfigured || activeProfile?.state === 'READY',
      errors: [],
      warnings,
    },
  }
}

function projectSnapshot(
  prov: VivyProvidersResult,
  settings: VivySettingsResult,
): ProviderSettingsSnapshot {
  const providers: ProviderSpec[] = []
  const entryByName: Record<string, VivyProviderEntryResult> = {}
  const matchedEntryIds = new Set<string>()

  const catalogSpecs = (vendor: VivyCatalogEntry) => {
    const multi = vendor.endpoints.length > 1
    for (const endpoint of vendor.endpoints) {
      const entry = prov.entries.find((e) =>
        endpointMatchesEntry(e, endpoint.adapter, endpoint.base_url),
      )
      if (entry) matchedEntryIds.add(entry.id)
      const name = multi ? `${vendor.vendor}#${endpoint.adapter}` : vendor.vendor
      providers.push(
        buildSpec(
          name,
          multi ? `${vendor.display_name} (${endpoint.adapter})` : vendor.display_name,
          endpoint.adapter,
          endpoint.base_url,
          endpoint.default_model,
          endpoint.models,
          entry,
          'builtin',
          endpoint.executable,
        ),
      )
      if (entry) entryByName[name] = entry
    }
  }
  prov.catalog.forEach(catalogSpecs)

  for (const entry of prov.entries) {
    if (matchedEntryIds.has(entry.id)) continue
    providers.push(
      buildSpec(
        entry.id,
        entry.display_name || entry.id,
        entry.bundle,
        entry.base_url,
        entry.default_model || null,
        [],
        entry,
        'custom',
        true,
      ),
    )
    entryByName[entry.id] = entry
  }

  const providerConfigs: Record<string, ProviderConfigEntry> = {}
  const keySetByName: Record<string, boolean> = {}
  for (const spec of providers) {
    const entry = entryByName[spec.name]
    if (entry) {
      providerConfigs[spec.name] = {
        apiKey: '',
        apiBase: entry.base_url,
        source: spec.source === 'custom' ? 'custom_providers' : 'providers',
      }
      keySetByName[spec.name] = entry.api_key_set
    } else {
      keySetByName[spec.name] = false
    }
  }

  return {
    providers,
    statusReport: buildStatusReport(providers, entryByName, prov, settings),
    providerConfigs,
    runtime: {
      provider: prov.active_provider || prov.config_provider || '',
      apiBase: prov.active_base_url || '',
      model: prov.active_model || prov.config_model || '',
      apiKeySet: settings.api_key_set,
    },
    entryByName,
    keySetByName,
    readOnly: prov.read_only || settings.read_only,
    frozen: prov.frozen || settings.frozen,
  }
}

let lastSnapshot: ProviderSettingsSnapshot | null = null

/** Loads `settings/providers` + `settings/get` and projects the view model. */
export async function loadProviderState(): Promise<ProviderSettingsSnapshot> {
  const [prov, settings] = await Promise.all([
    vivyClient.settingsProviders(),
    vivyClient.settingsGet(),
  ])
  lastSnapshot = projectSnapshot(prov, settings)
  return lastSnapshot
}

const findSpec = (snapshot: ProviderSettingsSnapshot, name: string) =>
  snapshot.providers.find((p) => p.name === name)

async function specFor(name: string): Promise<{
  snapshot: ProviderSettingsSnapshot
  spec: ProviderSpec
}> {
  const snapshot = lastSnapshot ?? (await loadProviderState())
  const spec = findSpec(snapshot, name)
  if (!spec) throw new Error(`unknown provider: ${name}`)
  return { snapshot, spec }
}

/** Materializes (or locates) the registry entry for a spec. */
async function ensureEntry(
  spec: ProviderSpec,
  overrides: { apiBase?: string } = {},
): Promise<VivyProviderEntryResult> {
  const prov = await vivyClient.settingsProviders()
  const baseUrl = overrides.apiBase ?? spec.default_api_base
  const byIdentity =
    prov.entries.find((e) => e.id === spec.name) ??
    prov.entries.find((e) =>
      endpointMatchesEntry(e, spec.api_type, baseUrl),
    )
  if (byIdentity) return byIdentity
  return vivyClient.providerUpsert({
    bundle: spec.api_type,
    base_url: baseUrl,
    display_name: spec.display_name,
    default_model: spec.default_model ?? undefined,
    models: [...spec.models, ...spec.custom_models],
  })
}

/**
 * Live model-list fetch: `settings/providers/refresh` performs GET /models
 * upstream with the backend-resolved credential and persists the result
 * (union-merged into the registry entry). OpenAI-compatible endpoints only.
 */
export async function getProviderModels(
  providerName: string,
  _apiBase?: string | null,
  _apiKey?: string | null,
): Promise<ProviderModelCatalog> {
  const { snapshot, spec } = await specFor(providerName)
  const entry = snapshot.entryByName[spec.name]
  const saved = entry
    ? await vivyClient.providerRefresh({ id: entry.id })
    : await vivyClient.providerRefresh({
        bundle: spec.api_type,
        base_url: spec.default_api_base,
        display_name: spec.display_name,
        default_model: spec.default_model ?? undefined,
      })
  return { models: saved.models, custom_models: [] }
}

/**
 * Connectivity check: the backend has no completion-probe RPC, so a "test" is
 * `providers/refresh` — a live GET /models proves reachability + credential.
 * A typed apiKey/apiBase is persisted to the registry first (VIVY owns
 * credentials; there is no ephemeral-credential probe path).
 */
export async function testProviderModel(
  providerName: string,
  _model: string,
  apiBase?: string | null,
  apiKey?: string | null,
): Promise<ProviderModelTestResult> {
  const started = Date.now()
  try {
    const { spec } = await specFor(providerName)
    const apiBaseOverride =
      apiBase && apiBase !== spec.default_api_base ? apiBase : undefined
    let entry = await ensureEntry(spec, apiBaseOverride ? { apiBase: apiBaseOverride } : {})
    const patch: ProviderUpsertParams = { id: entry.id, bundle: entry.bundle, base_url: entry.base_url }
    if (apiKey) patch.api_key = apiKey
    if (apiBaseOverride && apiBaseOverride !== entry.base_url) patch.base_url = apiBaseOverride
    if (patch.api_key !== undefined || patch.base_url !== entry.base_url) {
      entry = await vivyClient.providerUpsert(patch)
    }
    const refreshed = await vivyClient.providerRefresh({ id: entry.id })
    return {
      ok: true,
      message: `${refreshed.models.length} models`,
      latency_ms: Date.now() - started,
    }
  } catch (e) {
    return {
      ok: false,
      message: e instanceof Error ? e.message : String(e),
      latency_ms: Date.now() - started,
    }
  }
}

export async function addProviderModel(
  providerName: string,
  model: string,
): Promise<ProviderModelCatalog> {
  const { spec } = await specFor(providerName)
  const entry = await ensureEntry(spec)
  const models = entry.models.includes(model)
    ? entry.models
    : [...entry.models, model]
  const saved = await vivyClient.providerUpsert({ id: entry.id, models })
  return { models: saved.models, custom_models: saved.models }
}

export async function removeProviderModel(
  providerName: string,
  model: string,
): Promise<ProviderModelCatalog> {
  const { spec } = await specFor(providerName)
  const entry = await ensureEntry(spec)
  const saved = await vivyClient.providerUpsert({
    id: entry.id,
    models: entry.models.filter((m) => m !== model),
  })
  return { models: saved.models, custom_models: saved.models }
}

export async function createCustomProvider(
  payload: CustomProviderPayload,
): Promise<ProviderSpec> {
  const entry = await vivyClient.providerUpsert({
    id: payload.id || undefined,
    display_name: payload.displayName,
    bundle: 'openai-completions',
    base_url: payload.apiBase,
    default_model: payload.defaultModel,
    models: payload.models ?? (payload.defaultModel ? [payload.defaultModel] : []),
    ...(payload.apiKey ? { api_key: payload.apiKey } : {}),
  })
  return buildSpec(
    entry.id,
    entry.display_name,
    entry.bundle,
    entry.base_url,
    entry.default_model,
    [],
    entry,
    'custom',
    true,
  )
}

export async function deleteCustomProvider(providerName: string): Promise<void> {
  const snapshot = lastSnapshot ?? (await loadProviderState())
  const entry = snapshot.entryByName[providerName]
  if (!entry) throw new Error(`provider has no registry entry: ${providerName}`)
  await vivyClient.providerDelete(entry.id)
}

export interface ActiveProviderSelection {
  provider: string
  model: string
  apiBase?: string
  apiKey?: string
}

/**
 * Persists the active provider/model plus any new credential/base_url.
 * Order: materialize entry → write key/base_url via upsert →
 * `settings/model/select` (atomic; -32009 while runs are active).
 */
export async function saveActiveProvider(
  selection: ActiveProviderSelection,
): Promise<void> {
  const { spec } = await specFor(selection.provider)
  const apiBase = selection.apiBase?.trim()
  const entry = await ensureEntry(
    spec,
    apiBase ? { apiBase } : {},
  )
  const patch: {
    id: string
    api_key?: string
    base_url?: string
    display_name?: string
  } = { id: entry.id }
  const apiKey = selection.apiKey?.trim()
  if (apiKey) patch.api_key = apiKey
  if (apiBase && apiBase !== entry.base_url) patch.base_url = apiBase
  const saved =
    patch.api_key !== undefined || patch.base_url !== undefined
      ? await vivyClient.providerUpsert(patch as never)
      : entry
  await vivyClient.modelSelect({
    provider: saved.bundle,
    model: selection.model || saved.default_model,
    base_url: saved.base_url,
  })
  lastSnapshot = null
}

// ---------------------------------------------------------------------------
// DN-3 slice B — tools / MCP / skills / marketplace / network / compaction
// ---------------------------------------------------------------------------
import type {
  ChannelUpdateParams,
  VivyChannelEnvelope,
  VivyChannelStatus,
  VivyCronJob,
  VivyMcpServer,
  VivyNetworkSearchProvider,
  VivySkillSummary,
  VivyToolsResult,
} from './vivy/contracts'

/** One MCP server as the settings UI edits it. `env_from` maps a child-side
 * variable name to a HOST environment variable name — VIVY stores variable
 * names only, never credential values. */
export interface McpServerSpec {
  name: string
  enabled: boolean
  transport: 'stdio' | 'http'
  endpoint: string
  command: string
  args: string[]
  envFrom: Record<string, string>
  cwd: string
  authEnv: string
  resourceBridge: boolean
  /** Live state: ready | unavailable | unconfigured | deferred | inactive |
   * not_compiled. */
  state: string
  deferredReason: string
  envMissing: string[]
  authEnvSet: boolean
  toolCount: number
  error: string
}

export interface McpListState {
  servers: McpServerSpec[]
  readOnly: boolean
}

function toMcpSpec(server: VivyMcpServer): McpServerSpec {
  return {
    name: server.name,
    enabled: server.enabled,
    transport: server.transport === 'http' ? 'http' : 'stdio',
    endpoint: server.endpoint ?? '',
    command: server.command ?? '',
    args: server.args ?? [],
    envFrom: server.env_from ?? {},
    cwd: server.cwd ?? '',
    authEnv: server.auth_env ?? '',
    resourceBridge: server.resource_bridge,
    state: server.state,
    deferredReason: server.deferred_reason ?? '',
    envMissing: server.env_missing ?? [],
    authEnvSet: server.auth_env_set,
    toolCount: server.tool_count,
    error: server.error ?? '',
  }
}

function specToUpsert(spec: McpServerSpec) {
  // settings/mcp/upsert replaces the whole named entry — always send the
  // complete spec so a partial edit never wipes stored fields.
  return {
    name: spec.name,
    transport: spec.transport,
    endpoint: spec.transport === 'http' ? spec.endpoint.trim() : '',
    command: spec.transport === 'stdio' ? spec.command.trim() : '',
    args: spec.transport === 'stdio' ? spec.args : [],
    env_from: spec.transport === 'stdio' ? spec.envFrom : {},
    cwd: spec.cwd.trim(),
    auth_env: spec.authEnv.trim(),
    resource_bridge: spec.resourceBridge,
    enabled: spec.enabled,
  }
}

export async function loadMcpServers(): Promise<McpListState> {
  const result = await vivyClient.mcpList()
  return { servers: result.servers.map(toMcpSpec), readOnly: result.read_only }
}

/** Create or update a server; the returned spec carries fresh live status. */
export async function saveMcpServer(spec: McpServerSpec): Promise<McpServerSpec> {
  const saved = await vivyClient.mcpUpsert(specToUpsert(spec))
  return toMcpSpec(saved)
}

/** Toggle one server: upsert is full-replace, so the stored entry is merged
 * with the flipped flag before being re-sent. */
export async function setMcpServerEnabled(name: string, enabled: boolean): Promise<void> {
  const { servers } = await loadMcpServers()
  const target = servers.find((s) => s.name.toLowerCase() === name.toLowerCase())
  if (!target) throw new Error(`mcp server ${name} not found`)
  await vivyClient.mcpUpsert(specToUpsert({ ...target, enabled }))
}

export async function deleteMcpServer(name: string): Promise<void> {
  await vivyClient.mcpDelete(name)
}

/** Live connectivity probe (tools listing) for one configured server. */
export async function probeMcpServer(name: string): Promise<McpServerSpec> {
  return toMcpSpec(await vivyClient.mcpProbe(name))
}

// --- skills ---

export interface InstalledSkill {
  name: string
  description: string
  origin: string
  enabled: boolean
  /** Compare-and-swap base for set-enabled. */
  hash: string
  warnings: string[]
  userInvocable: boolean
}

function toInstalledSkill(item: VivySkillSummary): InstalledSkill {
  return {
    name: item.name,
    description: item.description,
    origin: item.origin ?? 'workspace',
    enabled: item.enabled,
    hash: item.hash,
    warnings: item.warnings ?? [],
    userInvocable: item.user_invocable ?? false,
  }
}

export async function loadInstalledSkills(): Promise<InstalledSkill[]> {
  const result = await vivyClient.skillsList()
  return result.skills.map(toInstalledSkill)
}

/** Flip a skill's enabled flag under CAS on its content hash. A -32009
 * conflict means the document changed under the caller — re-list and retry. */
export async function setSkillEnabled(
  name: string,
  enabled: boolean,
  baseHash: string,
): Promise<InstalledSkill> {
  return toInstalledSkill(await vivyClient.skillSetEnabled(name, enabled, baseHash))
}

// --- tools catalog / network search / compaction ---

export interface NetworkToolsState {
  /** Saved provider preference; '' means automatic. */
  searchProvider: string
  configProvider: string
  /** Backend roster in preference order; env_key + configured describe the
   * host-env credential presence — VIVY has no UI key-write surface. */
  providers: VivyNetworkSearchProvider[]
  /** Whether the network_search tool is in the effective active set. */
  searchEnabled: boolean
  /** Whether the web_fetch tool is in the effective active set. */
  fetchEnabled: boolean
}

const NETWORK_SEARCH_TOOL = 'network_search'
const WEB_FETCH_TOOL = 'web_fetch'

export async function loadNetworkToolsState(): Promise<NetworkToolsState> {
  const [settings, tools] = await Promise.all([
    vivyClient.settingsGet(),
    vivyClient.toolsList(),
  ])
  const search = settings.network_search
  const active = new Set(tools.active)
  return {
    searchProvider: search?.provider ?? '',
    configProvider: search?.config_provider ?? '',
    providers: search?.providers ?? [],
    searchEnabled: active.has(NETWORK_SEARCH_TOOL),
    fetchEnabled: active.has(WEB_FETCH_TOOL),
  }
}

/** Saves the search provider preference and the two tool toggles in one
 * tools/set-active write. Empty provider clears back to automatic. */
export async function saveNetworkToolsState(next: {
  searchProvider: string
  searchEnabled: boolean
  fetchEnabled: boolean
}): Promise<void> {
  await vivyClient.settingsUpdate({
    network_search: { provider: next.searchProvider.trim() },
  })
  const tools = await vivyClient.toolsList()
  const active = new Set(tools.active)
  if (next.searchEnabled) active.add(NETWORK_SEARCH_TOOL)
  else active.delete(NETWORK_SEARCH_TOOL)
  if (next.fetchEnabled) active.add(WEB_FETCH_TOOL)
  else active.delete(WEB_FETCH_TOOL)
  await vivyClient.toolsSetActive([...active])
}

export interface CompactionConfigShape {
  enabled: boolean
  maxTokens: number
  triggerPercent: number
  keepRecent: number
  configEnabled: boolean
  configMaxTokens: number
  configTriggerPercent: number
  configKeepRecent: number
}

export async function loadCompactionConfig(): Promise<CompactionConfigShape> {
  const settings = await vivyClient.settingsGet()
  const c = settings.compaction
  return {
    enabled: c?.enabled ?? true,
    maxTokens: c?.max_tokens ?? 0,
    triggerPercent: c?.trigger_percent ?? 80,
    keepRecent: c?.keep_recent ?? 12,
    configEnabled: c?.config_enabled ?? true,
    configMaxTokens: c?.config_max_tokens ?? 0,
    configTriggerPercent: c?.config_trigger_percent ?? 80,
    configKeepRecent: c?.config_keep_recent ?? 12,
  }
}

/** Merge-writes the compaction overlay; explicit zeros keep the config
 * values on the backend. */
export async function saveCompactionConfig(cfg: {
  enabled: boolean
  maxTokens: number
  triggerPercent: number
  keepRecent: number
}): Promise<void> {
  await vivyClient.settingsUpdate({
    compaction: {
      enabled: cfg.enabled,
      max_tokens: cfg.maxTokens,
      trigger_percent: cfg.triggerPercent,
      keep_recent: cfg.keepRecent,
    },
  })
}

/** Display-only budget shape for the context-budget pressure estimate. The
 * legacy system_budget_ratio has no VIVY knob — the estimator keeps its
 * default. */
export function budgetShapeFromCompaction(cfg: CompactionConfigShape): {
  max_tokens: number
  system_budget_ratio: number
  compact_threshold_ratio: number
  keep_recent_count: number
} {
  return {
    max_tokens: cfg.maxTokens,
    system_budget_ratio: 0.15,
    compact_threshold_ratio: cfg.triggerPercent / 100,
    keep_recent_count: cfg.keepRecent,
  }
}

export type { VivyToolsResult }

// ---- slice C: channels ----

export interface ChannelView {
  name: string
  /** Document truth (channel/get folded with the settings overlay). */
  envelope: VivyChannelEnvelope
  /** Process truth (channel/inspect); null when the channel has no status row. */
  status: VivyChannelStatus | null
  /** True when the document differs from what is running — applies on restart. */
  pendingRestart: boolean
}

export async function loadChannelsState(): Promise<ChannelView[]> {
  const statuses = await vivyClient.channelInspect()
  const views = await Promise.all(
    statuses.map(async (status) => {
      const envelope = await vivyClient.channelGet(status.name)
      const pendingRestart =
        envelope.enabled !== status.enabled ||
        envelope.token_env !== status.token_env ||
        JSON.stringify([...envelope.allow_from].sort()) !== JSON.stringify([...status.allow_from].sort())
      return { name: status.name, envelope, status, pendingRestart }
    }),
  )
  return views
}

export async function saveChannel(params: ChannelUpdateParams): Promise<VivyChannelEnvelope> {
  return vivyClient.channelUpdate(params)
}

// ---- slice C: cron ----

export async function listCronJobs(): Promise<VivyCronJob[]> {
  return (await vivyClient.cronList()).jobs
}
