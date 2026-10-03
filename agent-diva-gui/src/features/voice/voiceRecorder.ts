/**
 * Live recorder: getUserMedia → MediaRecorder → decode → C2-4 WAV.
 * Owns its tracks and AudioContext; every exit (stop/cancel/dispose)
 * releases them — no pet-path imports.
 */
import { encodeWav } from './encodeWav'
import type { VoiceRecorder } from './useVoiceController'

export function createMediaRecorder(deps?: {
  getUserMedia?: (constraints: MediaStreamConstraints) => Promise<MediaStream>
  decode?: (bytes: ArrayBuffer) => Promise<AudioBuffer>
}): VoiceRecorder {
  let stream: MediaStream | null = null
  let recorder: MediaRecorder | null = null
  let chunks: Blob[] = []
  let cancelled = false
  let stopResolve: ((wav: ArrayBuffer) => void) | null = null
  let stopReject: ((err: Error) => void) | null = null

  const getMedia = deps?.getUserMedia ?? ((c) => navigator.mediaDevices.getUserMedia(c))
  const decode = deps?.decode ?? ((bytes) => new AudioContext().decodeAudioData(bytes))

  function releaseTracks(): void {
    if (stream) {
      for (const track of stream.getTracks()) track.stop()
      stream = null
    }
  }

  return {
    start() {
      cancelled = false
      void (async () => {
        stream = await getMedia({ audio: true })
        recorder = new MediaRecorder(stream)
        chunks = []
        recorder.ondataavailable = (event) => {
          if (event.data.size > 0) chunks.push(event.data)
        }
        recorder.onstop = () => {
          void (async () => {
            try {
              if (cancelled || chunks.length === 0) {
                stopReject?.(new Error('recording cancelled'))
                return
              }
              const blob = new Blob(chunks)
              const decoded = await decode(await blob.arrayBuffer())
              stopResolve?.(encodeWav(decoded))
            } catch (err) {
              stopReject?.(err instanceof Error ? err : new Error(String(err)))
            } finally {
              releaseTracks()
              stopResolve = null
              stopReject = null
            }
          })()
        }
        recorder.onerror = () => {
          releaseTracks()
          stopReject?.(new Error('recorder error'))
          stopResolve = null
          stopReject = null
        }
        recorder.start()
      })().catch(() => {
        releaseTracks()
        stopReject?.(new Error('microphone unavailable'))
        stopResolve = null
        stopReject = null
      })
    },

    stop(): Promise<ArrayBuffer> {
      return new Promise((resolve, reject) => {
        if (!recorder || recorder.state === 'inactive') {
          releaseTracks()
          reject(new Error('recorder not running'))
          return
        }
        stopResolve = resolve
        stopReject = reject
        recorder.stop()
      })
    },

    cancel() {
      cancelled = true
      if (recorder && recorder.state !== 'inactive') recorder.stop()
      else releaseTracks()
    },

    dispose() {
      this.cancel()
      recorder = null
      releaseTracks()
    },
  }
}
