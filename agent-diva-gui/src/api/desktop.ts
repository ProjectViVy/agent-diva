import { invoke } from "@tauri-apps/api/core";

export interface GatewayProcessStatus {
  running: boolean;
  pid?: number | null;
  executable_path?: string | null;
  details?: string | null;
}

export interface SkillDto {
  name: string;
  description: string;
  source: 'builtin' | 'workspace';
  available: boolean;
  active: boolean;
  path: string;
  can_delete: boolean;
}

export interface McpConnectionStatusDto {
  state: 'connected' | 'degraded' | 'disabled' | 'invalid' | string;
  connected: boolean;
  applied: boolean;
  tool_count: number;
  error?: string | null;
  checked_at?: string | null;
}

export interface McpServerDto {
  name: string;
  enabled: boolean;
  transport: 'stdio' | 'http' | 'invalid' | string;
  command: string;
  args: string[];
  env: Record<string, string>;
  url: string;
  tool_timeout: number;
  status: McpConnectionStatusDto;
}

export interface McpServerPayload {
  name: string;
  enabled: boolean;
  command: string;
  args: string[];
  env: Record<string, string>;
  url: string;
  tool_timeout: number;
}

export interface StatusPathReport {
  config_path: string;
  config_dir: string;
  runtime_dir: string;
  workspace: string;
  cron_store: string;
  bridge_dir: string;
  whatsapp_auth_dir: string;
  whatsapp_media_dir: string;
}

export interface StatusDoctorSummary {
  valid: boolean;
  ready: boolean;
  errors: string[];
  warnings: string[];
}

export interface ProviderStatusSummary {
  name: string;
  display_name: string;
  default_model?: string | null;
  configurable: boolean;
  configured: boolean;
  ready: boolean;
  uses_api_base: boolean;
  provider_for_default_model: boolean;
  current: boolean;
  model?: string | null;
  api_base?: string | null;
  missing_fields: string[];
}

export interface ChannelStatusSummary {
  name: string;
  enabled: boolean;
  ready: boolean;
  missing_fields: string[];
  notes: string[];
}

export interface ConfigStatusReport {
  config: StatusPathReport;
  default_model: string;
  default_provider?: string | null;
  logging: {
    level: string;
    format: string;
    dir: string;
  };
  providers: ProviderStatusSummary[];
  channels: ChannelStatusSummary[];
  cron_jobs: number;
  mcp_servers: {
    configured: number;
    disabled: number;
  };
  doctor: StatusDoctorSummary;
}

export interface RuntimeConfigSnapshot {
  provider?: string | null;
  api_base?: string | null;
  model: string;
  has_api_key: boolean;
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

export const getConfigStatus = () =>
  invoke<ConfigStatusReport>("get_config_status");

export interface WipeSummary {
  removedPaths: string[];
}

export const wipeLocalData = () => invoke<WipeSummary>("wipe_local_data");

export const getRuntimeConfig = () =>
  invoke<RuntimeConfigSnapshot>("get_config");

export const saveRawConfig = (raw: string) =>
  invoke<void>("save_config", { raw });

export const tailLogs = (lines: number) =>
  invoke<string[]>("tail_logs", { lines });

export const checkHealth = () => invoke<boolean>("check_health");

export const getSkills = () => invoke<SkillDto[]>("get_skills");

export const getMcps = () => invoke<McpServerDto[]>("get_mcps");

export const createMcp = (payload: McpServerPayload) =>
  invoke<McpServerDto>("create_mcp", { payload });

export const updateMcp = (name: string, payload: McpServerPayload) =>
  invoke<McpServerDto>("update_mcp", { name, payload });

export const deleteMcp = (name: string) =>
  invoke<void>("delete_mcp", { name });

export const setMcpEnabled = (name: string, enabled: boolean) =>
  invoke<McpServerDto>("set_mcp_enabled", { name, enabled });

export const refreshMcpStatus = (name: string) =>
  invoke<McpServerDto>("refresh_mcp_status", { name });

export const uploadSkill = (fileName: string, bytes: number[]) =>
  invoke<SkillDto>("upload_skill", { fileName, bytes });

export const deleteSkill = (name: string) =>
  invoke<void>("delete_skill", { name });

// ============================================================
// Marketplace API (skills.sh)
// ============================================================

export interface MarketplaceSkillEntry {
  id: string;
  name: string;
  description: string;
  tags: string[];
  category: string;
  trustLevel: "official" | "certified" | "community";
  installCount: number;
  starCount: number;
  author: string;
  installUrl: string;
  repoUrl?: string;
}

export interface MarketplaceSearchResult {
  skills: MarketplaceSkillEntry[];
  total: number;
  page: number;
  hasMore: boolean;
}

export interface MarketplaceSearchParams {
  query?: string;
  category?: string;
  trustLevel?: string;
  sort?: "installs" | "rating" | "newest" | "stars";
  page?: number;
  limit?: number;
}

const MARKETPLACE_BASE_URL = "https://skills.sh/api";

export async function searchMarketplaceSkills(
  params: MarketplaceSearchParams
): Promise<MarketplaceSearchResult> {
  const url = new URL(`${MARKETPLACE_BASE_URL}/search`);
  if (params.query) url.searchParams.set("q", params.query);
  if (params.category) url.searchParams.set("category", params.category);
  if (params.trustLevel) url.searchParams.set("trustLevel", params.trustLevel);
  if (params.sort) url.searchParams.set("sort", params.sort);
  url.searchParams.set("page", String(params.page ?? 1));
  url.searchParams.set("limit", String(params.limit ?? 20));

  const response = await fetch(url.toString());
  if (!response.ok)
    throw new Error(`Marketplace search failed: ${response.status}`);
  return response.json();
}

export async function installSkillFromUrl(
  installUrl: string,
  onProgress?: (status: string) => void
): Promise<SkillDto> {
  onProgress?.("downloading");
  const response = await fetch(installUrl);
  if (!response.ok)
    throw new Error(`Failed to download skill: ${response.status}`);

  const buffer = await response.arrayBuffer();
  const bytes = Array.from(new Uint8Array(buffer));
  const fileName = installUrl.split("/").pop() || "skill.zip";

  onProgress?.("installing");
  const result = await uploadSkill(fileName, bytes);
  return result;
}
