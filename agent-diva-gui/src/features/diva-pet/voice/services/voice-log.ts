import { computed, ref } from 'vue'

export type VoiceLogLevel = 'info' | 'warn' | 'error'
export type VoiceLogSource = 'asr' | 'tts' | 'settings' | 'vrm-animation'

export interface VoiceLogEvent {
  id: number
  at: number
  level: VoiceLogLevel
  source: VoiceLogSource
  message: string
  detail?: Record<string, string | number | boolean | null>
}

const MAX_EVENTS = 300
const events = ref<VoiceLogEvent[]>([])
let nextId = 1

function sanitizeDetail(
  detail?: Record<string, string | number | boolean | null | undefined>,
): Record<string, string | number | boolean | null> | undefined {
  if (!detail) return undefined

  const sanitized: Record<string, string | number | boolean | null> = {}
  for (const [key, value] of Object.entries(detail)) {
    const lowerKey = key.toLowerCase()
    if (lowerKey.includes('apikey') || lowerKey.includes('api_key') || lowerKey.includes('authorization')) {
      sanitized[key] = value ? '[redacted]' : null
      continue
    }
    sanitized[key] = value ?? null
  }
  return sanitized
}

export function addVoiceLogEvent(event: Omit<VoiceLogEvent, 'id' | 'at'>): void {
  events.value = [
    {
      ...event,
      id: nextId++,
      at: Date.now(),
      detail: sanitizeDetail(event.detail),
    },
    ...events.value,
  ].slice(0, MAX_EVENTS)
}

export function clearVoiceLogEvents(): void {
  events.value = []
}

export function useVoiceLog() {
  return {
    events: computed(() => events.value),
    recentEvents: computed(() => events.value.slice(0, 100)),
    clear: clearVoiceLogEvents,
  }
}
