import { describe, expect, it } from 'vitest'
import { NativeUnavailableError, speechConfig, speechNative, voiceAssets } from './speech'

// DN-6C step 5: outside the tauri shell every speech surface reports the
// honest native_unavailable error — no fake browser path, no crash.
describe('speech facade (browser mode)', () => {
  it('rejects config reads with native_unavailable', async () => {
    await expect(speechConfig.get()).rejects.toBeInstanceOf(NativeUnavailableError)
    await expect(speechConfig.get()).rejects.toMatchObject({ code: 'native_unavailable' })
  })

  it('rejects controller-native requests with native_unavailable', async () => {
    const identity = { request_id: 'r', session_id: 's', utterance_id: 'u', generation: 1 }
    await expect(speechNative.synthesize(identity, 'text')).rejects.toMatchObject({
      code: 'native_unavailable',
    })
    await expect(speechNative.transcribe(identity, new ArrayBuffer(4))).rejects.toMatchObject({
      code: 'native_unavailable',
    })
    await expect(speechNative.contextSet('s', 2)).rejects.toMatchObject({ code: 'native_unavailable' })
  })

  it('rejects asset and credential lanes with native_unavailable', async () => {
    await expect(voiceAssets.list()).rejects.toMatchObject({ code: 'native_unavailable' })
    await expect(voiceAssets.delete('a')).rejects.toMatchObject({ code: 'native_unavailable' })
  })
})
