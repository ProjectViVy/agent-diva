import { describe, expect, it } from 'vitest'
import { resolveGaussSceneUrl } from './gauss-scene'

describe('resolveGaussSceneUrl', () => {
  it('adds a leading slash for public asset paths', () => {
    expect(resolveGaussSceneUrl('vrm/scene/home.spz')).toBe('/vrm/scene/home.spz')
  })

  it('preserves existing absolute public paths', () => {
    expect(resolveGaussSceneUrl('/vrm/scene/home.spz')).toBe('/vrm/scene/home.spz')
  })

  it('preserves external and blob urls', () => {
    expect(resolveGaussSceneUrl('https://example.test/home.spz')).toBe('https://example.test/home.spz')
    expect(resolveGaussSceneUrl('blob:http://localhost/id')).toBe('blob:http://localhost/id')
  })

  it('returns undefined for empty paths', () => {
    expect(resolveGaussSceneUrl('')).toBeUndefined()
    expect(resolveGaussSceneUrl('   ')).toBeUndefined()
    expect(resolveGaussSceneUrl(null)).toBeUndefined()
  })
})
