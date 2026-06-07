import { describe, expect, it, vi } from 'vitest'
import { ExpressionController } from './expression-controller'

describe('ExpressionController', () => {
  it('clears all mapped expressions when mood returns to neutral', () => {
    const setValue = vi.fn()
    const controller = new ExpressionController()

    controller.attach({
      expressionManager: {
        setValue,
      },
    } as any)

    setValue.mockClear()
    controller.setMood('happy')
    controller.setMood('neutral')

    const neutralCalls = setValue.mock.calls.slice(-5)
    expect(neutralCalls).toEqual([
      ['neutral', 0],
      ['happy', 0],
      ['sad', 0],
      ['angry', 0],
      ['surprised', 0],
    ])
  })
})
