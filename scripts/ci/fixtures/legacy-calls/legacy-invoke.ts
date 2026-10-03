import { invoke } from '@tauri-apps/api/core'
export const send = () => invoke('send_message', { text: 'hi' })
