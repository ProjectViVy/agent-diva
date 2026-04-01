import { invoke } from "@tauri-apps/api/core";

/** 与 Rust `cortex_sync::ERR_CORTEX_SYNC_REJECTED` 一致；`invoke` 失败时用于 i18n 映射（Story 2.2）。 */
export const CORTEX_SYNC_REJECTED = "cortex_sync_rejected";

/** 与 `docs/swarm-cortex-contract-v0.md` 及 Rust `CortexState`（camelCase JSON）一致。 */
export interface CortexState {
  enabled: boolean;
  schemaVersion: number;
}

export const getCortexState = () => invoke<CortexState>("get_cortex_state");

export const setCortexEnabled = (enabled: boolean) =>
  invoke<CortexState>("set_cortex_enabled", { enabled });

export const toggleCortex = () => invoke<CortexState>("toggle_cortex");
