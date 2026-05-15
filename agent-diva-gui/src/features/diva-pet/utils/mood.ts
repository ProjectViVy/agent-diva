import type { PetMessage, VrmMood } from '../types'

const MOOD_KEYWORDS: Record<Exclude<VrmMood, 'neutral'>, string[]> = {
  happy: [
    '哈哈',
    '开心',
    '太好了',
    '喜欢',
    'great',
    'happy',
    'love',
    'excellent',
    'wonderful',
    '恭喜',
  ],
  sad: [
    '难过',
    '伤心',
    '遗憾',
    'sorry',
    'unfortunately',
    '失望',
    '不行',
  ],
  angry: [
    '生气',
    '可恶',
    '愤怒',
    'damn',
    'frustrating',
    '讨厌',
  ],
  surprised: [
    '哇',
    '天哪',
    '真的吗',
    'wow',
    'amazing',
    'incredible',
    '惊讶',
  ],
}

const EMOTION_TO_MOOD: Record<string, VrmMood> = {
  happy: 'happy',
  sad: 'sad',
  angry: 'angry',
  surprised: 'surprised',
  normal: 'neutral',
  neutral: 'neutral',
  clingy: 'happy',
  jealous: 'angry',
}

export function normalizeMood(input?: string | null): VrmMood {
  if (!input) {
    return 'neutral'
  }

  const normalized = input.trim().toLowerCase()
  return EMOTION_TO_MOOD[normalized] ?? 'neutral'
}

export function deriveMoodFromText(text?: string | null): VrmMood {
  if (!text) {
    return 'neutral'
  }

  const normalized = text.toLowerCase()
  for (const [mood, keywords] of Object.entries(MOOD_KEYWORDS) as [VrmMood, string[]][]) {
    if (keywords.some((keyword) => normalized.includes(keyword))) {
      return mood
    }
  }

  return 'neutral'
}

export function deriveMoodFromMessages(messages: PetMessage[], fallbackEmotion?: string | null): VrmMood {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index]
    if (message.role !== 'agent') {
      continue
    }

    const detected = deriveMoodFromText(message.content)
    if (detected !== 'neutral') {
      return detected
    }
  }

  return normalizeMood(fallbackEmotion)
}
