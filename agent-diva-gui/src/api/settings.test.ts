import { beforeEach, describe, expect, it, vi } from 'vitest'

const calls: Array<{ method: string; params: unknown }> = []
const results = vi.hoisted(() => ({
  providers: null as unknown,
  settings: null as unknown,
  store: new Map<string, Record<string, unknown>>(),
}))

vi.mock('./vivy/instance', () => ({
  vivyClient: {
    settingsProviders: vi.fn(async () => results.providers),
    settingsGet: vi.fn(async () => results.settings),
    // Mimics the backend merge: an update-by-id keeps stored fields when the
    // patch leaves them empty (empty api_key never wipes a credential).
    providerUpsert: vi.fn(async (params: Record<string, unknown>) => {
      calls.push({ method: 'settings/providers/upsert', params })
      const id = (params.id as string) ?? 'custom-x1'
      const prev = results.store.get(id) ?? {
        display_name: 'entry',
        bundle: 'openai-completions',
        base_url: 'https://x/v1',
        default_model: 'm1',
        models: ['m1'],
        api_key_set: false,
      }
      const next = {
        id,
        display_name: params.display_name ?? prev.display_name,
        bundle: params.bundle ?? prev.bundle,
        base_url: params.base_url ?? prev.base_url,
        default_model: params.default_model ?? prev.default_model,
        models: params.models ?? prev.models,
        api_key_set: params.api_key ? true : prev.api_key_set,
      }
      results.store.set(id, next)
      return next
    }),
    providerDelete: vi.fn(async () => ({ deleted: true })),
    providerRefresh: vi.fn(async () => ({
      id: 'custom-x1',
      display_name: 'x',
      bundle: 'openai-completions',
      base_url: 'https://x/v1',
      default_model: 'm1',
      models: ['m1', 'm2'],
      api_key_set: true,
    })),
    modelSelect: vi.fn(async (params: unknown) => {
      calls.push({ method: 'settings/model/select', params })
      return results.providers
    }),
  },
}))

import {
  loadProviderState,
  saveActiveProvider,
  addProviderModel,
} from './settings'

const catalog = [
  {
    vendor: 'deepseek',
    display_name: 'DeepSeek',
    endpoints: [
      {
        adapter: 'openai-completions',
        base_url: 'https://api.deepseek.com/v1',
        default_model: 'deepseek-chat',
        models: ['deepseek-chat', 'deepseek-reasoner'],
        executable: true,
        state: 'SUPPORTED',
      },
    ],
  },
  {
    vendor: 'sensenova',
    display_name: 'SenseNova',
    endpoints: [
      {
        adapter: 'openai-completions',
        base_url: 'https://token.sensenova.cn/v1',
        default_model: 'sensenova-6.8-flash-lite',
        models: ['sensenova-6.8-flash-lite'],
        executable: true,
        state: 'SUPPORTED',
      },
    ],
  },
]

const entry = {
  id: 'prov-sensenova',
  display_name: 'SenseNova',
  bundle: 'openai-completions',
  base_url: 'https://token.sensenova.cn/v1',
  default_model: 'sensenova-6.8-flash-lite',
  models: ['sensenova-6.8-flash-lite', 'sensenova-6.8-pro'],
  api_key_set: true,
}

function baseSettings(overrides: Record<string, unknown> = {}) {
  return {
    provider: 'openai-completions',
    default_model: 'sensenova-6.8-flash-lite',
    base_url: 'https://token.sensenova.cn/v1',
    api_key_set: true,
    frozen: false,
    read_only: false,
    config_provider: 'deepseek',
    config_model: 'deepseek-chat',
    provider_profiles: [
      { id: 'openai-completions', adapter_family: 'openai-completions', endpoint_class: 'native', model_ids: [], state: 'READY' },
    ],
    ...overrides,
  }
}

function baseProviders(overrides: Record<string, unknown> = {}) {
  return {
    entries: [entry],
    catalog,
    profiles: [
      { id: 'openai-completions', adapter_family: 'openai-completions', endpoint_class: 'native', model_ids: [], state: 'READY' },
    ],
    active_provider: 'openai-completions',
    active_model: 'sensenova-6.8-flash-lite',
    active_base_url: 'https://token.sensenova.cn/v1',
    read_only: false,
    frozen: false,
    config_provider: 'deepseek',
    config_model: 'deepseek-chat',
    ...overrides,
  }
}

describe('settings adapter (DN-3 slice A)', () => {
  beforeEach(() => {
    calls.length = 0
    results.providers = baseProviders()
    results.settings = baseSettings()
    results.store.clear()
    results.store.set('prov-sensenova', {
      id: 'prov-sensenova',
      display_name: 'SenseNova',
      bundle: 'openai-completions',
      base_url: 'https://token.sensenova.cn/v1',
      default_model: 'sensenova-6.8-flash-lite',
      models: ['sensenova-6.8-flash-lite', 'sensenova-6.8-pro'],
      api_key_set: true,
    })
  })

  it('projects catalog + registry entries into provider specs', async () => {
    const snap = await loadProviderState()
    const names = snap.providers.map((p) => p.name)
    expect(names).toContain('deepseek')
    expect(names).toContain('sensenova')

    const sensenova = snap.providers.find((p) => p.name === 'sensenova')!
    expect(sensenova.api_type).toBe('openai-completions')
    expect(sensenova.models).toContain('sensenova-6.8-pro') // entry overlay union
    expect(sensenova.custom_models).toEqual(['sensenova-6.8-pro'])
    expect(snap.entryByName['sensenova'].id).toBe('prov-sensenova')
  })

  it('redacts credentials: only api_key_set reaches the view model', async () => {
    const snap = await loadProviderState()
    expect(snap.keySetByName['sensenova']).toBe(true)
    expect(snap.keySetByName['deepseek']).toBe(false)
    for (const cfg of Object.values(snap.providerConfigs)) {
      expect(cfg.apiKey).toBe('')
    }
  })

  it('marks the active provider via bundle+base_url, not name collisions', async () => {
    const snap = await loadProviderState()
    const sensenovaStatus = snap.statusReport.providers.find((p) => p.name === 'sensenova')!
    const deepseekStatus = snap.statusReport.providers.find((p) => p.name === 'deepseek')!
    expect(sensenovaStatus.current).toBe(true)
    expect(sensenovaStatus.ready).toBe(true)
    expect(deepseekStatus.current).toBe(false)
    expect(deepseekStatus.configured).toBe(false)
    expect(deepseekStatus.missing_fields).toEqual(['api_key'])
    expect(snap.statusReport.doctor.ready).toBe(true)
  })

  it('materializes a registry entry then selects model for a keyed save', async () => {
    const snap = await loadProviderState()
    expect(snap.entryByName['deepseek']).toBeUndefined()

    await saveActiveProvider({
      provider: 'deepseek',
      model: 'deepseek-chat',
      apiBase: 'https://api.deepseek.com/v1',
      apiKey: 'sk-test',
    })

    const upsert = calls.find((c) => c.method === 'settings/providers/upsert')!
    expect(upsert.params).toMatchObject({
      bundle: 'openai-completions',
      base_url: 'https://api.deepseek.com/v1',
    })
    const select = calls.find((c) => c.method === 'settings/model/select')!
    expect(select.params).toMatchObject({
      model: 'deepseek-chat',
      base_url: 'https://api.deepseek.com/v1',
    })
    // upsert must happen before select
    expect(calls.indexOf(upsert)).toBeLessThan(calls.indexOf(select))
  })

  it('writes a replacement key through the entry before re-selecting', async () => {
    await loadProviderState()
    await saveActiveProvider({
      provider: 'sensenova',
      model: 'sensenova-6.8-pro',
      apiKey: 'sk-new',
    })
    const upsert = calls.find((c) => c.method === 'settings/providers/upsert')!
    expect(upsert.params).toMatchObject({ id: 'prov-sensenova', api_key: 'sk-new' })
    const select = calls.find((c) => c.method === 'settings/model/select')!
    expect(select.params).toMatchObject({ model: 'sensenova-6.8-pro' })
  })

  it('addProviderModel patches the entry models list', async () => {
    await loadProviderState()
    await addProviderModel('sensenova', 'm3')
    const upsert = calls.find((c) => c.method === 'settings/providers/upsert')!
    expect(upsert.params).toMatchObject({
      id: 'prov-sensenova',
      models: expect.arrayContaining(['sensenova-6.8-flash-lite', 'sensenova-6.8-pro', 'm3']),
    })
  })
})
