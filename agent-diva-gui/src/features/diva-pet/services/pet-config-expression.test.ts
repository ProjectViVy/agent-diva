import { describe, expect, it, vi } from 'vitest'
import { applyExpressionDefaultMigration, migrateConfig } from './pet-config'

describe('applyExpressionDefaultMigration', () => {
  it('enables vrmExpressionEnabled once when the migration key is missing', () => {
    const getItem = vi.fn(() => null)
    const setItem = vi.fn()

    const migrated = applyExpressionDefaultMigration(
      migrateConfig({ vrmExpressionEnabled: false }),
      getItem,
      setItem,
    )

    expect(migrated.vrmExpressionEnabled).toBe(true)
    expect(setItem).toHaveBeenCalledWith('agent-diva-pet-expression-default-enabled-migrated-v1', '1')
  })

  it('does not override vrmExpressionEnabled after the migration key exists', () => {
    const getItem = vi.fn(() => '1')
    const setItem = vi.fn()

    const migrated = applyExpressionDefaultMigration(
      migrateConfig({ vrmExpressionEnabled: false }),
      getItem,
      setItem,
    )

    expect(migrated.vrmExpressionEnabled).toBe(false)
    expect(setItem).not.toHaveBeenCalled()
  })
})
