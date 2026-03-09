import { invoke } from "@tauri-apps/api/core";

export interface GatewayProcessStatus {
  running: boolean;
  pid?: number | null;
  executable_path?: string | null;
  details?: string | null;
}

export const isTauriRuntime = () =>
  typeof window !== "undefined" &&
  ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

export const getGatewayProcessStatus = () =>
  invoke<GatewayProcessStatus>("get_gateway_process_status");

export const startGateway = (binPath?: string | null) =>
  invoke<void>("start_gateway", { binPath: binPath ?? null });

export const stopGateway = () => invoke<void>("stop_gateway");

export const loadRawConfig = () => invoke<string>("load_config");

export const saveRawConfig = (raw: string) =>
  invoke<void>("save_config", { raw });

export const tailLogs = (lines: number) =>
  invoke<string[]>("tail_logs", { lines });

export const checkHealth = () => invoke<boolean>("check_health");
