import { deriveMoodFromText } from '../features/diva-mate/utils/mood'

interface DesktopMateEmotionMessage {
  role: 'user' | 'agent' | 'system' | 'tool'
  content: string
  isStreaming?: boolean
  timestamp?: number
  fromHistory?: boolean
}

export interface DesktopMateEmotionSignal {
  signature: string
  mood: string
}

export function getDesktopMateEmotionSignal(
  messages: DesktopMateEmotionMessage[],
): DesktopMateEmotionSignal | null {
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
