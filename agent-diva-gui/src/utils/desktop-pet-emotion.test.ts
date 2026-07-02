import { describe, expect, it } from 'vitest'
import { getDesktopPetEmotionSignal } from './desktop-pet-emotion'

describe('getDesktopPetEmotionSignal', () => {
  it('creates one stable signature for a completed agent reply', () => {
    const signal = getDesktopPetEmotionSignal([
      { role: 'user', content: 'hello', timestamp: 1 },
      { role: 'agent', content: 'I am happy to help.', timestamp: 2 },
    ])

    expect(signal).toEqual({
      signature: '2:I am happy to help.',
      mood: 'happy',
    })
  })

  it('ignores streaming agent placeholders so partial updates do not emit moods', () => {
    const signal = getDesktopPetEmotionSignal([
      { role: 'agent', content: 'I am happy', timestamp: 2, isStreaming: true },
    ])

    expect(signal).toBeNull()
  })

  it('ignores historical agent messages when restoring old sessions', () => {
    const signal = getDesktopPetEmotionSignal([
      { role: 'agent', content: 'I am happy to help.', timestamp: 2, fromHistory: true },
    ])

    expect(signal).toBeNull()
  })
})
