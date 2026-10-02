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
