// negative fixture: a literal invoke inside the frozen desktop-host seam that
// is NOT in the fixed native command allowlist must be denied. Scanned under
// the INVOKE_FILE rel path by the selftest (seam-* prefix convention).
import { invoke } from '@tauri-apps/api/core'
export const tail = () => invoke('tail_logs')
