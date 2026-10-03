import { invoke as callBackend } from '@tauri-apps/api/core'
export const run = () => callBackend('get_sessions')
