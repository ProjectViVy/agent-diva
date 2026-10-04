/**
 * DN-6C WAV encoder: mic capture → decoded AudioBuffer → mono 16 kHz
 * PCM16 WAV, the single input shape `speech_transcribe` accepts (C2-4:
 * PCM16/mono/16kHz, <=120 s). Resampling is linear interpolation — the
 * same pipeline DN-0S verified end-to-end on Chrome 137 Web Audio.
 */

export const TRANSCRIBE_SAMPLE_RATE = 16000
export const TRANSCRIBE_MAX_SECONDS = 120
export const TRANSCRIBE_MAX_BYTES = 8 * 1024 * 1024

/** Resample one Float32 channel to `targetRate` by linear interpolation. */
export function resampleMono(samples: Float32Array, sourceRate: number, targetRate: number): Float32Array {
  if (sourceRate === targetRate) return samples.slice()
  if (samples.length === 0 || sourceRate <= 0) return new Float32Array(0)
  const ratio = sourceRate / targetRate
  const outLength = Math.max(1, Math.floor(samples.length / ratio))
  const out = new Float32Array(outLength)
  for (let i = 0; i < outLength; i += 1) {
    const pos = i * ratio
    const left = Math.floor(pos)
    const right = Math.min(left + 1, samples.length - 1)
    const frac = pos - left
    out[i] = samples[left] * (1 - frac) + samples[right] * frac
  }
  return out
}

/** Downmix an arbitrary channel layout to one mono channel. */
export function downmixToMono(channels: Float32Array[]): Float32Array {
  if (channels.length === 0) return new Float32Array(0)
  if (channels.length === 1) return channels[0]
  const length = channels[0].length
  const out = new Float32Array(length)
  for (const channel of channels) {
    for (let i = 0; i < length; i += 1) out[i] += channel[i]
  }
  for (let i = 0; i < length; i += 1) out[i] /= channels.length
  return out
}

/** Float32 [-1,1] → little-endian PCM16. */
export function floatToPcm16(samples: Float32Array): Int16Array {
  const out = new Int16Array(samples.length)
  for (let i = 0; i < samples.length; i += 1) {
    const s = Math.max(-1, Math.min(1, samples[i]))
    out[i] = s < 0 ? s * 0x8000 : s * 0x7fff
  }
  return out
}

/** Wrap PCM16 samples in a RIFF/WAVE container (mono, `sampleRate`). */
export function encodeWavBytes(pcm: Int16Array, sampleRate: number): ArrayBuffer {
  const dataBytes = pcm.length * 2
  const buffer = new ArrayBuffer(44 + dataBytes)
  const view = new DataView(buffer)
  const writeStr = (offset: number, text: string) => {
    for (let i = 0; i < text.length; i += 1) view.setUint8(offset + i, text.charCodeAt(i))
  }
  writeStr(0, 'RIFF')
  view.setUint32(4, 36 + dataBytes, true)
  writeStr(8, 'WAVE')
  writeStr(12, 'fmt ')
  view.setUint32(16, 16, true) // PCM fmt chunk
  view.setUint16(20, 1, true) // PCM format
  view.setUint16(22, 1, true) // mono
  view.setUint32(24, sampleRate, true)
  view.setUint32(28, sampleRate * 2, true) // byte rate
  view.setUint16(32, 2, true) // block align
  view.setUint16(34, 16, true) // bits
  writeStr(36, 'data')
  view.setUint32(40, dataBytes, true)
  new Int16Array(buffer, 44).set(pcm)
  return buffer
}

export interface DecodedAudio {
  sampleRate: number
  numberOfChannels: number
  getChannelData(channel: number): Float32Array
}

/**
 * Decoded audio → C2-4 WAV bytes. Truncation is never silent: an overlong
 * capture throws so the caller drops the request instead of shipping a
 * clipped transcript upstream.
 */
export function encodeWav(decoded: DecodedAudio): ArrayBuffer {
  const seconds = decoded.getChannelData(0).length / decoded.sampleRate
  if (seconds > TRANSCRIBE_MAX_SECONDS) {
    throw new Error(`recording exceeds ${TRANSCRIBE_MAX_SECONDS}s bound`)
  }
  const channels = Array.from({ length: decoded.numberOfChannels }, (_, i) => decoded.getChannelData(i))
  const mono = downmixToMono(channels)
  const resampled = resampleMono(mono, decoded.sampleRate, TRANSCRIBE_SAMPLE_RATE)
  const wav = encodeWavBytes(floatToPcm16(resampled), TRANSCRIBE_SAMPLE_RATE)
  if (wav.byteLength > TRANSCRIBE_MAX_BYTES) {
    throw new Error('recording exceeds the 8 MiB transcribe bound')
  }
  return wav
}
