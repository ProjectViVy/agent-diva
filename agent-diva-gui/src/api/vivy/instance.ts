/**
 * Shared VIVY client instance. The client is a stateless RPC wrapper over
 * the Tauri transport; components that only issue calls (no projection)
 * may import this instead of building their own client.
 */
import { VivyClient } from './client'
import { createTauriTransport } from './transport'

export const vivyClient = new VivyClient(createTauriTransport())
