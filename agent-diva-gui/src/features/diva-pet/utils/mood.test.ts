import { describe, expect, it } from 'vitest'
import { deriveMoodFromMessages } from './mood'
import type { PetMessage } from '../types'

describe('deriveMoodFromMessages', () => {
  it('returns the latest agent mood when the newest agent message matches a keyword', () => {
    const messages: PetMessage[] = [
      { role: 'user', content: 'hello' },
      { role: 'agent', content: 'I am happy to help.' },
    ]

    expect(deriveMoodFromMessages(messages)).toBe('happy')
  })

  it('does not persist an older non-neutral mood when the latest agent message is neutral', () => {
    const messages: PetMessage[] = [
      { role: 'agent', content: 'I am happy to help.' },
      { role: 'user', content: 'next question' },
      { role: 'agent', content: 'Here is the answer.' },
    ]

    expect(deriveMoodFromMessages(messages)).toBe('neutral')
  })

  it('falls back to neutral for normal or neutral fallback emotions', () => {
    expect(deriveMoodFromMessages([], 'normal')).toBe('neutral')
    expect(deriveMoodFromMessages([], 'neutral')).toBe('neutral')
  })
})
