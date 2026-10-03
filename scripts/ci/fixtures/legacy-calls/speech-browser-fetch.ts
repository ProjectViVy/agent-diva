// negative fixture: speech provider traffic must go through native
// speech_* commands — a browser fetch to a provider host is denied twice
// (fetch rule + provider endpoint literal rule).
export const transcribe = (wav: ArrayBuffer) =>
  fetch('https://api.siliconflow.cn/v1/audio/transcriptions', { method: 'POST', body: wav })
