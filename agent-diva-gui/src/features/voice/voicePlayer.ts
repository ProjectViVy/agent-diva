/**
 * Live player: MP3 bytes → Blob URL → Audio element. Owns the object URL —
 * created only after identity-fenced bytes arrive, revoked on stop/dispose.
 */
import type { VoicePlayer } from './useVoiceController'

export function createAudioPlayer(deps?: { audioFactory?: () => HTMLAudioElement }): VoicePlayer {
  const makeAudio = deps?.audioFactory ?? (() => new Audio())
  let audio: HTMLAudioElement | null = null
  let objectUrl: string | null = null

  function release(): void {
    if (audio) {
      audio.pause()
      audio.src = ''
      audio = null
    }
    if (objectUrl) {
      URL.revokeObjectURL(objectUrl)
      objectUrl = null
    }
  }

  return {
    play(bytes, _utteranceId) {
      release()
      return new Promise<void>((resolve, reject) => {
        objectUrl = URL.createObjectURL(new Blob([bytes], { type: 'audio/mpeg' }))
        audio = makeAudio()
        audio.src = objectUrl
        audio.onended = () => {
          release()
          resolve()
        }
        audio.onerror = () => {
          release()
          reject(new Error('playback failed'))
        }
        void audio.play().catch((err) => {
          release()
          reject(err instanceof Error ? err : new Error(String(err)))
        })
      })
    },
    stop: release,
    dispose: release,
  }
}
