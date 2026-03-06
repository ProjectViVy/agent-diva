import { invoke } from "@tauri-apps/api/core";

export interface RuntimeInfo {
  platform: string;
  is_bundled: boolean;
  resource_dir?: string | null;
}

export interface ServiceStatusPayload {
  installed: boolean;
  running: boolean;
  state: string;
  executable_path?: string | null;
  details?: string | null;
}

export interface GatewayProcessStatus {
  running: boolean;
  pid?: number | null;
  executable_path?: string | null;
  details?: string | null;
}

export const isTauriRuntime = () =>
  typeof window !== "undefined" &&
  ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

export const getRuntimeInfo = () => invoke<RuntimeInfo>("get_runtime_info");

export const getServiceStatus = () =>
  invoke<ServiceStatusPayload>("get_service_status");

export const installService = () => invoke<void>("install_service");

export const uninstallService = () => invoke<void>("uninstall_service");

export const startService = () => invoke<void>("start_service");

export const stopService = () => invoke<void>("stop_service");

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
