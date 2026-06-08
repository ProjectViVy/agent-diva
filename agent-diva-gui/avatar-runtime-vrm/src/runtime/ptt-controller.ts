/**
 * PDF-Controller — push-to-talk microphone recording and WebM→WAV PCM16 encoding.
 *
 * Manages the complete lifecycle: getUserMedia → MediaRecorder capture →
 * WebM Blob → 16kHz mono PCM16 WAV → base64 data URL.
 * Uses the same Hooks pattern as MotionController/AnimationController.
 *
 * Reference: super-agent-party/static/js/vrm.js lines 2508–2540 (pttEncodeWav),
 * 2544–2627 (setupPttInteraction recording logic).
 */

/** Callbacks for PttController lifecycle events. */
export interface PttControllerHooks {
  /** Called when recording state changes (start/stop). */
  onStateChange?: (state: { recording: boolean }) => void | Promise<void>
  /** Called when audio encoding completes. payload.audioDataUrl is a base64 data URL. */
  onAudioReady?: (payload: { format: 'wav'; audioDataUrl: string }) => void | Promise<void>
  /** Called when transcription results are available (reserved for external integration). */
  onTranscription?: (payload: { text: string; isFinal: boolean }) => void | Promise<void>
  /** Called on recoverable or unrecoverable errors during capture/encoding. */
  onError?: (error: { code: string; message: string }) => void | Promise<void>
}

/**
 * Manages push-to-talk microphone recording using the MediaRecorder API.
 *
 * Encodes captured WebM audio to 16kHz mono PCM16 WAV format suitable for
 * speech recognition backends. Designed to be API-driven — any external UI
 * or key-binding layer calls `start()` / `stop()`.
 */
export class PttController {
  private readonly hooks: PttControllerHooks
  private mediaRecorder: MediaRecorder | null = null
  private audioChunks: Blob[] = []
  private stream: MediaStream | null = null
  private _recording = false
  private cancelled = false

  /**
   * @param hooks - Lifecycle callbacks (all optional). Follows the same pattern
   *                as {@link MotionControllerHooks}.
   */
  constructor(hooks: PttControllerHooks = {}) {
    this.hooks = hooks
  }

  // ---------------------------------------------------------------------------
  // Public API
  // ---------------------------------------------------------------------------

  /**
   * Request microphone access and begin recording.
   *
   * If `stop()` was called before `getUserMedia` resolved the stream is
   * released immediately without starting the MediaRecorder.
   */
  async start(): Promise<void> {
    if (this._recording) return

    this.cancelled = false
    this._recording = true
    this.audioChunks = []

    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true })

      // If stop() was called while waiting for the permission prompt,
      // release the stream and bail out.
      if (this.cancelled) {
        stream.getTracks().forEach((t) => t.stop())
        this._recording = false
        return
      }

      this.stream = stream

      const recorder = new MediaRecorder(stream)
      recorder.ondataavailable = (ev: BlobEvent) => {
        if (ev.data.size > 0) {
          this.audioChunks.push(ev.data)
        }
      }
      recorder.onstop = () => this.handleStop()
      recorder.start()
      this.mediaRecorder = recorder

      await this.invokeHook(this.hooks.onStateChange, { recording: true })
    } catch (err: unknown) {
      this._recording = false
      const message =
        err instanceof DOMException
          ? err.message
          : err instanceof Error
            ? err.message
            : 'Unknown microphone error'

      await this.invokeHook(this.hooks.onError, {
        code: 'MIC_ACCESS_DENIED',
        message,
      })
    }
  }

  /**
   * Stop the current recording, release the microphone, and encode audio.
   *
   * Safe to call at any time — if `start()` is still waiting for the
   * permission prompt the request will be cancelled.
   */
  stop(): void {
    this.cancelled = true
    if (!this._recording) return

    this._recording = false

    if (this.mediaRecorder && this.mediaRecorder.state === 'recording') {
      this.mediaRecorder.stop()
    }
  }

  /** Whether the controller is currently recording. */
  isRecording(): boolean {
    return this._recording
  }

  /**
   * Stop recording (if active) and release all resources.
   * The controller should be discarded after calling this.
   */
  destroy(): void {
    this.stop()
    this.mediaRecorder = null
    this.audioChunks = []
    if (this.stream) {
      this.stream.getTracks().forEach((t) => t.stop())
      this.stream = null
    }
  }

  // ---------------------------------------------------------------------------
  // Internal helpers
  // ---------------------------------------------------------------------------

  /** Called by MediaRecorder.onstop — encodes WebM→WAV and fires onAudioReady. */
  private async handleStop(): Promise<void> {
    try {
      // Release microphone tracks
      if (this.stream) {
        this.stream.getTracks().forEach((t) => t.stop())
        this.stream = null
      }

      await this.invokeHook(this.hooks.onStateChange, { recording: false })

      if (this.audioChunks.length === 0) return

      const webmBlob = new Blob(this.audioChunks, { type: 'audio/webm' })
      const wavBlob = await this.encodeWav(webmBlob)

      const audioDataUrl = await this.blobToDataUrl(wavBlob)
      await this.invokeHook(this.hooks.onAudioReady, {
        format: 'wav',
        audioDataUrl,
      })
    } catch (err: unknown) {
      const message =
        err instanceof Error ? err.message : 'Unknown encoding error'
      await this.invokeHook(this.hooks.onError, {
        code: 'ENCODE_FAILED',
        message,
      })
    }
  }

  /**
   * Encode a WebM audio blob to 16kHz mono PCM16 WAV.
   *
   * Reference implementation: super-agent-party vrm.js `pttEncodeWav`.
   */
  private async encodeWav(webmBlob: Blob): Promise<Blob> {
    // Create at 16kHz — the target sample rate for Sherpa-style ASR.
    const audioCtx = new AudioContext({ sampleRate: 16000 })
    try {
      const arrayBuffer = await webmBlob.arrayBuffer()
      const audioBuffer = await audioCtx.decodeAudioData(arrayBuffer)

      // Extract the first (mono) channel.
      const channelData = audioBuffer.getChannelData(0)
      const numSamples = channelData.length

      // WAV header (44 bytes) + PCM16 sample data
      const wavBuffer = new ArrayBuffer(44 + numSamples * 2)
      const view = new DataView(wavBuffer)

      // --- RIFF chunk ---
      this.writeString(view, 0, 'RIFF')
      view.setUint32(4, 36 + numSamples * 2, true) // file size - 8
      this.writeString(view, 8, 'WAVE')

      // --- fmt chunk ---
      this.writeString(view, 12, 'fmt ')
      view.setUint32(16, 16, true) // chunk size (PCM = 16)
      view.setUint16(20, 1, true) // audio format (1 = PCM)
      view.setUint16(22, 1, true) // channels (mono)
      view.setUint32(24, 16000, true) // sample rate
      view.setUint32(28, 16000 * 2, true) // byte rate (sampleRate * channels * bytesPerSample)
      view.setUint16(32, 2, true) // block align (channels * bytesPerSample)
      view.setUint16(34, 16, true) // bits per sample

      // --- data chunk ---
      this.writeString(view, 36, 'data')
      view.setUint32(40, numSamples * 2, true) // data chunk size

      // --- PCM16 samples (float32 → int16) ---
      for (let i = 0; i < numSamples; i++) {
        const clamped = Math.max(-1, Math.min(1, channelData[i]))
        // Reference: s < 0 ? s * 0x8000 : s * 0x7FFF
        view.setInt16(
          44 + i * 2,
          clamped < 0 ? clamped * 0x8000 : clamped * 0x7fff,
          true,
        )
      }

      return new Blob([wavBuffer], { type: 'audio/wav' })
    } finally {
      audioCtx.close()
    }
  }

  /** Convert a Blob to a base64 data URL using FileReader. */
  private blobToDataUrl(blob: Blob): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader()
      reader.onloadend = () => resolve(reader.result as string)
      reader.onerror = () => reject(new Error('FileReader failed'))
      reader.readAsDataURL(blob)
    })
  }

  /** Write an ASCII string into a DataView at the given byte offset. */
  private writeString(view: DataView, offset: number, str: string): void {
    for (let i = 0; i < str.length; i++) {
      view.setUint8(offset + i, str.charCodeAt(i))
    }
  }

  /**
   * Safely invoke an optional hook callback, handling both sync and async
   * (Promise-returning) handlers.
   */
  private async invokeHook<T extends unknown[]>(
    fn: ((...args: T) => void | Promise<void>) | undefined,
    ...args: T
  ): Promise<void> {
    if (!fn) return
    try {
      await fn(...args)
    } catch {
      // Silently swallow hook errors to prevent one misbehaving callback
      // from breaking the controller's internal state machine.
    }
  }
}

export default PttController
