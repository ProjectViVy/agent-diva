import { invoke } from '@tauri-apps/api/core'
const cmd = 'get_' + 'sessions'
export const run = () => invoke(cmd)
