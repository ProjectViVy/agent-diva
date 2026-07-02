import { deriveMoodFromText } from '../features/diva-pet/utils/mood'

interface DesktopPetEmotionMessage {
  role: 'user' | 'agent' | 'system' | 'tool'
  content: string
  isStreaming?: boolean
  timestamp?: number
  fromHistory?: boolean
}

export interface DesktopPetEmotionSignal {
  signature: string
  mood: string
}

export function getDesktopPetEmotionSignal(
  messages: DesktopPetEmotionMessage[],
): DesktopPetEmotionSignal | null {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index]
    if (message.role !== 'agent') {
      continue
    }

    const content = message.content.trim()
    if (!content || message.isStreaming || message.fromHistory) {
      return null
    }

    return {
      signature: `${message.timestamp ?? index}:${content}`,
      mood: deriveMoodFromText(content),
    }
  }

  return null
}
