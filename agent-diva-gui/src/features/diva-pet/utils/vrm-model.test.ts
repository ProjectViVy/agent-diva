import { describe, expect, it } from 'vitest'
import { isCustomVrmModelPath, resolveVrmModelPath, toVrmModelId } from './vrm-model'

describe('vrm-model utils', () => {
  it('uses the default bundled model when config is empty', () => {
    expect(resolveVrmModelPath('')).toBe('/vrm/models/Alice.vrm')
  })

  it('appends the vrm extension only when missing', () => {
    expect(resolveVrmModelPath('Alice')).toBe('/vrm/models/Alice.vrm')
    expect(resolveVrmModelPath('Alice.vrm')).toBe('/vrm/models/Alice.vrm')
  })

  it('preserves already-prefixed public paths', () => {
    expect(resolveVrmModelPath('/vrm/models/Alice.vrm')).toBe('/vrm/models/Alice.vrm')
    expect(resolveVrmModelPath('vrm/models/Alice.vrm')).toBe('/vrm/models/Alice.vrm')
  })

  it('normalizes stored values back to a comparable model id', () => {
    expect(toVrmModelId('Alice')).toBe('Alice')
    expect(toVrmModelId('Alice.vrm')).toBe('Alice')
    expect(toVrmModelId('/vrm/models/Alice.vrm')).toBe('Alice')
  })

  it('preserves custom model paths under the user config VRM directory', () => {
    const path = 'vrm/models/custom/MyModel.vrm'
    expect(resolveVrmModelPath(path)).toBe(path)
    expect(isCustomVrmModelPath(path)).toBe(true)
    expect(toVrmModelId(path)).toBe('MyModel')
  })
})
