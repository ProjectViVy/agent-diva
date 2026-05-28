import { describe, expect, it, vi } from 'vitest'
import { ref } from 'vue'
import { DEFAULT_PET_CONFIG, type PetConfig, type VrmAppearanceConfig } from '../types'
import {
  useAppearanceConfig,
  generateAppearanceId,
  createEmptyAppearance,
} from '../services/appearance-config'
import { DEFAULT_VRM_MODEL_PATH } from '../utils/default-appearance'

// ── Helpers ────────────────────────────────────────────────

function makeAppearance(overrides?: Partial<VrmAppearanceConfig>): VrmAppearanceConfig {
  return {
    id: generateAppearanceId(),
    name: 'Test Appearance',
    modelId: 'test-model',
    motionIds: [],
    expressionEnabled: true,
    motionEnabled: true,
    ...overrides,
  }
}

function setup() {
  const config = ref<PetConfig>({ ...DEFAULT_PET_CONFIG })
  const updateConfig = vi.fn((patch: Partial<PetConfig>) => {
    config.value = { ...config.value, ...patch }
  })

  const api = useAppearanceConfig(config, updateConfig)

  return { config, updateConfig, api }
}

// ── Tests ──────────────────────────────────────────────────

describe('useAppearanceConfig', () => {
  // ── createAppearance ─────────────────────────────────────

  describe('createAppearance', () => {
    it('adds an appearance to the list', () => {
      const { api } = setup()
      const appearance = makeAppearance({ name: 'Test 1' })

      api.createAppearance(appearance)

      expect(api.appearances.value).toHaveLength(1)
      expect(api.appearances.value[0].name).toBe('Test 1')
    })

    it('preserves existing appearances when adding', () => {
      const { api } = setup()

      const a1 = makeAppearance({ name: 'First', id: 'id-1' })
      const a2 = makeAppearance({ name: 'Second', id: 'id-2' })

      api.createAppearance(a1)
      api.createAppearance(a2)

      expect(api.appearances.value).toHaveLength(2)
      expect(api.appearances.value[0].name).toBe('First')
      expect(api.appearances.value[1].name).toBe('Second')
    })

    it('calls updateConfig with the new list', () => {
      const { api, updateConfig } = setup()
      const appearance = makeAppearance({ name: 'New' })

      api.createAppearance(appearance)

      expect(updateConfig).toHaveBeenCalledWith(
        expect.objectContaining({ vrmAppearances: expect.any(Array) }),
      )
    })
  })

  // ── updateAppearance ─────────────────────────────────────

  describe('updateAppearance', () => {
    it('patches the name field', () => {
      const { api } = setup()
      const appearance = makeAppearance({ name: 'Original', id: 'fixed-id' })
      api.createAppearance(appearance)

      api.updateAppearance('fixed-id', { name: 'Updated' })

      expect(api.appearances.value[0].name).toBe('Updated')
    })

    it('patches motionIds field', () => {
      const { api } = setup()
      const appearance = makeAppearance({ name: 'Test', id: 'fixed-id', motionIds: [] })
      api.createAppearance(appearance)

      api.updateAppearance('fixed-id', { motionIds: ['m1', 'm2'] })

      expect(api.appearances.value[0].motionIds).toEqual(['m1', 'm2'])
    })

    it('patches multiple fields at once', () => {
      const { api } = setup()
      const appearance = makeAppearance({
        name: 'Original',
        id: 'fixed-id',
        expressionEnabled: true,
        motionEnabled: true,
      })
      api.createAppearance(appearance)

      api.updateAppearance('fixed-id', {
        name: 'Renamed',
        expressionEnabled: false,
      })

      const updated = api.appearances.value[0]
      expect(updated.name).toBe('Renamed')
      expect(updated.expressionEnabled).toBe(false)
      // Unpatched field preserved
      expect(updated.motionEnabled).toBe(true)
    })

    it('preserves unpatched fields', () => {
      const { api } = setup()
      const appearance = makeAppearance({
        name: 'Original',
        id: 'fixed-id',
        modelId: 'my-model',
      })
      api.createAppearance(appearance)

      api.updateAppearance('fixed-id', { name: 'Renamed' })

      const updated = api.appearances.value[0]
      expect(updated.modelId).toBe('my-model')
    })

    it('does nothing when ID is not found', () => {
      const { api } = setup()
      const appearance = makeAppearance({ id: 'real-id' })
      api.createAppearance(appearance)

      // Should not throw
      expect(() => api.updateAppearance('nonexistent', { name: 'X' })).not.toThrow()
      expect(api.appearances.value[0].name).not.toBe('X')
    })

    it('does not affect other appearances', () => {
      const { api } = setup()
      const a1 = makeAppearance({ name: 'First', id: 'id-1' })
      const a2 = makeAppearance({ name: 'Second', id: 'id-2' })
      api.createAppearance(a1)
      api.createAppearance(a2)

      api.updateAppearance('id-1', { name: 'First Updated' })

      expect(api.appearances.value[0].name).toBe('First Updated')
      expect(api.appearances.value[1].name).toBe('Second')
    })
  })

  // ── deleteAppearance ─────────────────────────────────────

  describe('deleteAppearance', () => {
    it('removes the appearance by ID', () => {
      const { api } = setup()
      const a1 = makeAppearance({ id: 'id-1' })
      const a2 = makeAppearance({ id: 'id-2' })
      api.createAppearance(a1)
      api.createAppearance(a2)

      expect(api.appearances.value).toHaveLength(2)

      api.deleteAppearance('id-1')
      expect(api.appearances.value).toHaveLength(1)
      expect(api.appearances.value[0].id).toBe('id-2')
    })

    it('falls back to default when deleting the active appearance', () => {
      const { api, config } = setup()
      const a1 = makeAppearance({ id: 'id-1' })
      const a2 = makeAppearance({ id: 'id-2' })
      api.createAppearance(a1)
      api.createAppearance(a2)

      // Set active to id-1
      config.value = { ...config.value, activeAppearanceId: 'id-1' }

      api.deleteAppearance('id-1')

      expect(api.activeId.value).toBe('default')
    })

    it('keeps activeId when deleting a non-active appearance', () => {
      const { api, config } = setup()
      const a1 = makeAppearance({ id: 'id-1' })
      const a2 = makeAppearance({ id: 'id-2' })
      api.createAppearance(a1)
      api.createAppearance(a2)

      config.value = { ...config.value, activeAppearanceId: 'id-1' }

      api.deleteAppearance('id-2')
      expect(api.activeId.value).toBe('id-1')
    })

    it('resets to default when deleting the last appearance', () => {
      const { api, config } = setup()
      const a1 = makeAppearance({ id: 'only-one' })
      api.createAppearance(a1)

      config.value = { ...config.value, activeAppearanceId: 'only-one' }

      api.deleteAppearance('only-one')

      // Last appearance can't be deleted — resets to default
      expect(api.appearances.value).toHaveLength(0)
      expect(api.activeId.value).toBe(DEFAULT_PET_CONFIG.activeAppearanceId)
    })

    it('does not throw when deleting from empty list', () => {
      const { api } = setup()

      expect(() => api.deleteAppearance('nothing')).not.toThrow()
    })
  })

  // ── switchAppearance ─────────────────────────────────────

  describe('switchAppearance', () => {
    it('changes activeId to the specified ID', () => {
      const { api } = setup()
      const a1 = makeAppearance({ id: 'id-1' })
      const a2 = makeAppearance({ id: 'id-2' })
      api.createAppearance(a1)
      api.createAppearance(a2)

      const result = api.switchAppearance('id-2')
      expect(result).toBe(true)
      expect(api.activeId.value).toBe('id-2')
    })

    it('returns false when already active', () => {
      const { api, config } = setup()
      const a1 = makeAppearance({ id: 'id-1' })
      api.createAppearance(a1)

      config.value = { ...config.value, activeAppearanceId: 'id-1' }

      const result = api.switchAppearance('id-1')
      expect(result).toBe(false)
      expect(api.activeId.value).toBe('id-1')
    })

    it('returns false when ID not found', () => {
      const { api } = setup()

      const result = api.switchAppearance('nonexistent')
      expect(result).toBe(false)
    })
  })

  // ── getActiveAppearance ──────────────────────────────────

  describe('getActiveAppearance', () => {
    it('returns the active appearance config', () => {
      const { api, config } = setup()
      const appearance = makeAppearance({ id: 'active-id', name: 'Active One' })
      api.createAppearance(appearance)

      config.value = { ...config.value, activeAppearanceId: 'active-id' }

      const active = api.getActiveAppearance()
      expect(active).toBeDefined()
      expect(active!.name).toBe('Active One')
    })

    it('returns the default appearance when no appearance matches activeId', () => {
      const { api, config } = setup()

      config.value = { ...config.value, activeAppearanceId: 'orphan-id' }

      const active = api.getActiveAppearance()
      expect(active.id).toBe('default')
      expect(active.modelId).toBe(DEFAULT_VRM_MODEL_PATH)
    })

    it('returns the default appearance when appearance list is empty', () => {
      const { api } = setup()

      const active = api.getActiveAppearance()
      expect(active.id).toBe('default')
    })
  })

  // ── findAppearance ───────────────────────────────────────

  describe('findAppearance', () => {
    it('finds an appearance by ID', () => {
      const { api } = setup()
      const appearance = makeAppearance({ id: 'find-me', name: 'Found' })
      api.createAppearance(appearance)

      const found = api.findAppearance('find-me')
      expect(found).toBeDefined()
      expect(found!.name).toBe('Found')
    })

    it('returns undefined for nonexistent ID', () => {
      const { api } = setup()
      const appearance = makeAppearance({ id: 'real-id' })
      api.createAppearance(appearance)

      const found = api.findAppearance('ghost-id')
      expect(found).toBeUndefined()
    })

    it('returns undefined when list is empty', () => {
      const { api } = setup()

      const found = api.findAppearance('anything')
      expect(found).toBeUndefined()
    })
  })

  // ── appearances (reactive) ───────────────────────────────

  describe('appearances reactive list', () => {
    it('reflects changes after create', () => {
      const { api } = setup()

      expect(api.appearances.value).toHaveLength(0)

      api.createAppearance(makeAppearance({ name: 'A' }))
      expect(api.appearances.value).toHaveLength(1)
    })

    it('reflects changes after update', () => {
      const { api } = setup()
      const appearance = makeAppearance({ id: 'test-id', name: 'Before' })
      api.createAppearance(appearance)

      api.updateAppearance('test-id', { name: 'After' })
      expect(api.appearances.value[0].name).toBe('After')
    })
  })

  // ── activeId (reactive) ──────────────────────────────────

  describe('activeId reactive', () => {
    it('starts with DEFAULT_PET_CONFIG.activeAppearanceId', () => {
      const { api } = setup()
      expect(api.activeId.value).toBe('default')
    })

    it('reflects changes after switchAppearance', () => {
      const { api } = setup()
      const appearance = makeAppearance({ id: 'switch-to-me' })
      api.createAppearance(appearance)

      api.switchAppearance('switch-to-me')
      expect(api.activeId.value).toBe('switch-to-me')
    })
  })
})

// ── generateAppearanceId ──────────────────────────────────

describe('generateAppearanceId', () => {
  it('returns a string with expected prefix', () => {
    const id = generateAppearanceId()
    expect(id).toMatch(/^appearance-\d+-[a-z0-9]+$/)
  })

  it('returns unique IDs on multiple calls', () => {
    const ids = new Set<string>()
    for (let i = 0; i < 100; i++) {
      ids.add(generateAppearanceId())
    }
    expect(ids.size).toBe(100)
  })
})

// ── createEmptyAppearance ─────────────────────────────────

describe('createEmptyAppearance', () => {
  it('returns an appearance config with the given name and modelId', () => {
    const appearance = createEmptyAppearance('My Model', 'model-123')

    expect(appearance.id).toBeDefined()
    expect(appearance.name).toBe('My Model')
    expect(appearance.modelId).toBe('model-123')
  })

  it('has empty motionIds by default', () => {
    const appearance = createEmptyAppearance('Test', 'm1')
    expect(appearance.motionIds).toEqual([])
  })

  it('has expressionEnabled and motionEnabled both true', () => {
    const appearance = createEmptyAppearance('Test', 'm1')
    expect(appearance.expressionEnabled).toBe(true)
    expect(appearance.motionEnabled).toBe(true)
  })

  it('has a unique ID matching the generate format', () => {
    const appearance = createEmptyAppearance('Test', 'm1')
    expect(appearance.id).toMatch(/^appearance-\d+-[a-z0-9]+$/)
  })
})
