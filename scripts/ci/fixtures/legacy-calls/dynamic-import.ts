export const run = async () => (await import('@tauri-apps/api/core')).invoke('tail_logs')
