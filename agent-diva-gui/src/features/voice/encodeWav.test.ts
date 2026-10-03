import { describe, expect, it } from 'vitest'
import {
  downmixToMono,
  encodeWav,
  encodeWavBytes,
  floatToPcm16,
  resampleMono,
  TRANSCRIBE_SAMPLE_RATE,
} from './encodeWav'

function wavHeader(bytes: ArrayBuffer) {
  const view = new DataView(bytes)
  const str = (offset: number, len: number) =>
    String.fromCharCode(...new Uint8Array(bytes, offset, len))
  return {
    riff: str(0, 4),
    wave: str(8, 4),
    fmt: str(12, 4),
    format: view.getUint16(20, true),
    channels: view.getUint16(22, true),
    sampleRate: view.getUint32(24, true),
    bits: view.getUint16(34, true),
    data: str(36, 4),
    dataBytes: view.getUint32(40, true),
  }
}

describe('encodeWav', () => {
  it('produces a mono 16kHz PCM16 RIFF container', () => {
    const pcm = new Int16Array([0, 16384, -16384, 32767, -32768])
    const bytes = encodeWavBytes(pcm, TRANSCRIBE_SAMPLE_RATE)
    const header = wavHeader(bytes)
    expect(header).toMatchObject({
      riff: 'RIFF', wave: 'WAVE', fmt: 'fmt ', format: 1,
      channels: 1, sampleRate: 16000, bits: 16, data: 'data',
      dataBytes: pcm.length * 2,
    })
    expect(bytes.byteLength).toBe(44 + pcm.length * 2)
    expect(new Int16Array(bytes, 44)).toEqual(pcm)
  })

  it('resamples 48kHz down to 16kHz mono at ~3x compression', () => {
    const seconds = 1
    const source = new Float32Array(48000 * seconds).fill(0.5)
    const out = resampleMono(source, 48000, TRANSCRIBE_SAMPLE_RATE)
    expect(out.length).toBe(16000)
    expect(out.every((v) => Math.abs(v - 0.5) < 1e-6)).toBe(true)
  })

  it('downmixes stereo to the channel average', () => {
    const left = new Float32Array([1, 1])
    const right = new Float32Array([-1, 0])
    const mono = downmixToMono([left, right])
    expect(Array.from(mono)).toEqual([0, 0.5])
  })

  it('clamps float samples into PCM16 range', () => {
    const pcm = floatToPcm16(new Float32Array([1.5, -1.5, 0.5, -0.5]))
    expect(pcm[0]).toBe(32767)
    expect(pcm[1]).toBe(-32768)
  })

  it('encodeWav round-trips a decoded stereo source', () => {
    const channel = new Float32Array(48000).fill(0.25)
    const decoded = {
      sampleRate: 48000,
      numberOfChannels: 2,
      getChannelData: () => channel,
    }
    const bytes = encodeWav(decoded)
    const header = wavHeader(bytes)
    expect(header.channels).toBe(1)
    expect(header.sampleRate).toBe(16000)
    expect(header.dataBytes).toBe(16000 * 2)
    const pcm = new Int16Array(bytes, 44)
    expect(pcm[0]).toBe(Math.trunc(0.25 * 0x7fff))
  })

  it('rejects overlong capture instead of silently truncating', () => {
    const decoded = {
      sampleRate: 16000,
      numberOfChannels: 1,
      getChannelData: () => new Float32Array(16000 * 121),
    }
    expect(() => encodeWav(decoded)).toThrow(/120s/)
  })
})
