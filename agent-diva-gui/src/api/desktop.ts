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

export interface FileAttachmentDto {
  file_id: string;
  filename: string;
  size: number;
  mime_type?: string | null;
  channel: string;
  message_id?: string | null;
  uploaded_by?: string | null;
  stored_at: string;
  ref_count: number;
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

export const uploadFile = (fileName: string, bytes: number[], channel: string, messageId?: string) =>
  invoke<FileAttachmentDto>("upload_file", { fileName, bytes, channel, messageId });

export const deleteSkill = (name: string) =>
  invoke<void>("delete_skill", { name });

// ============================================================
// Laputa / Evolution Governance API
// ============================================================

export type EvidenceSource =
  | 'session'
  | 'report'
  | 'auto_dream_run'
  | 'laputa_section'
  | 'user_input'
  | 'file'
  | 'context_compaction';

export interface EvidenceRef {
  id: string;
  source: EvidenceSource;
  uri: string;
  excerpt?: string | null;
  hash?: string | null;
  created_at: string;
}

export type ProposalType =
  | 'memory_patch'
  | 'journal_note'
  | 'learning_note'
  | 'identity_patch'
  | 'relationship_update'
  | 'commitment_set'
  | 'sop_create'
  | 'deprecation';

export type ProposalState =
  | 'pending_review'
  | 'approved'
  | 'rejected'
  | 'edited'
  | 'deferred'
  | 'applied'
  | 'reverted'
  | 'superseded'
  | 'needs_attention'
  | 'run_failed';

export type RiskLevel = 'low' | 'medium' | 'high' | 'critical';

export type LaputaSectionName =
  | 'identity'
  | 'relationship'
  | 'commitment'
  | 'preferences'
  | 'memory_md'
  | 'history_md'
  | 'daily'
  | 'weekly'
  | 'monthly'
  | 'journal_reflective'
  | 'proposal_inbox'
  | 'changelog'
  | 'report_indexes'
  | 'aaak_summaries';

export interface EvolutionProposal {
  id: string;
  created_at: string;
  updated_at: string;
  created_by: string;
  proposal_type: ProposalType;
  target_section: LaputaSectionName;
  evidence_refs: EvidenceRef[];
  proposed_patch: string;
  risk_level: RiskLevel;
  state: ProposalState;
  source_run_id?: string | null;
}

export interface LaputaSection {
  name: LaputaSectionName;
  status: string;
  content: unknown;
  metadata: Record<string, unknown>;
  last_modified: string;
  version: string;
}

export interface LaputaSnapshot {
  schema_version: string;
  sections: Record<string, LaputaSection>;
  changed_sections: string[];
  updated_at?: string | null;
  server_time: string;
}

export type ChangelogAction = 'apply' | 'revert' | 'rollback';

export interface ChangelogRecord {
  id: string;
  action: ChangelogAction;
  target_section: LaputaSectionName;
  before: string;
  after: string;
  diff: string;
  proposal_id?: string | null;
  audit_event_id?: string | null;
  reverted: boolean;
  stale: boolean;
  created_at: string;
  applied_by: string;
}

export interface ChangelogPage {
  items: ChangelogRecord[];
  total: number;
  page: number;
  page_size: number;
  has_more: boolean;
}

export interface RollbackOutcome {
  changelog: ChangelogRecord;
  audit_event: {
    id: string;
    kind: string;
    actor: string;
    proposal_id?: string | null;
    target_section?: LaputaSectionName | null;
    message: string;
    created_at: string;
  };
}

export interface ProposalApplyResult {
  proposal: EvolutionProposal;
  changelog: ChangelogRecord;
  audit_event: {
    id: string;
    kind: string;
    actor: string;
    proposal_id?: string | null;
    target_section?: LaputaSectionName | null;
    message: string;
    created_at: string;
  };
  rollback_request: {
    changelog_id: string;
    requested_by: string;
    reason: string;
    requested_at: string;
  };
}

export interface ProposalEditPayload {
  proposed_patch?: string | null;
  evidence_refs?: EvidenceRef[] | null;
  risk_level?: RiskLevel | null;
  updated_at?: string | null;
}

export interface ProposalTransitionPayload {
  state: ProposalState;
  updated_at?: string | null;
}

export interface ApplyProposalPayload {
  actor?: string | null;
  applied_at?: string | null;
}

export interface RollbackChangelogPayload {
  reason: string;
  expected_current?: string | null;
}

export type AutoDreamRunState = 'pending' | 'running' | 'cancelled' | 'completed' | 'failed';

export interface AutoDreamInputSourceSummary {
  source: string;
  included_items: number;
  total_bytes: number;
  truncated: boolean;
}

export interface AutoDreamInputOmission {
  source: string;
  detail: string;
}

export interface AutoDreamInputSummary {
  total_items: number;
  included_sources: AutoDreamInputSourceSummary[];
  omissions: AutoDreamInputOmission[];
  truncated: boolean;
  total_bytes: number;
}

export interface AutoDreamRunRecord {
  id: string;
  started_at: string;
  completed_at?: string | null;
  state: AutoDreamRunState;
  trigger: string;
  summary?: string | null;
  input_summary?: AutoDreamInputSummary | null;
  proposal_ids: string[];
  error?: string | null;
}

export interface SelfEvolutionConfig {
  enabled: boolean;
  autodream_frequency: 'daily' | 'weekly' | 'manual';
  trigger_threshold_sessions: number;
  trigger_threshold_messages: number;
  auto_merge_confidence: number;
  require_confirmation_for: string[];
}

export type LaputaEventKind = 'proposals' | 'changelog' | 'errors';

export interface LaputaEvent {
  id?: string;
  kind?: string;
  payload?: unknown;
  message?: string;
  created_at?: string;
  [key: string]: unknown;
}

export const listLaputaProposals = (since?: string) =>
  invoke<EvolutionProposal[]>("laputa_list_proposals", { since: since ?? null });

export const getLaputaProposal = (id: string) =>
  invoke<EvolutionProposal>("laputa_get_proposal", { id });

export const editLaputaProposal = (id: string, payload: ProposalEditPayload) =>
  invoke<EvolutionProposal>("laputa_edit_proposal", { id, payload });

export const transitionLaputaProposal = (id: string, payload: ProposalTransitionPayload) =>
  invoke<EvolutionProposal>("laputa_transition_proposal", { id, payload });

export const applyLaputaProposal = (id: string, payload: ApplyProposalPayload = {}) =>
  invoke<ProposalApplyResult>("laputa_apply_proposal", { id, payload });

export interface WriteLaputaSectionResult {
  changelog_id: string;
  applied_at: string;
  status?: string;
}

export const getLaputaSection = (name: LaputaSectionName) =>
  invoke<LaputaSection>("laputa_get_section", { name });

export const getLaputaSnapshot = (since?: string) =>
  invoke<LaputaSnapshot>("laputa_get_snapshot", { since: since ?? null });

export const writeLaputaSection = (
  name: LaputaSectionName,
  content: string,
  summary?: string,
) =>
  invoke<WriteLaputaSectionResult>("laputa_write_section", {
    name,
    content,
    summary: summary ?? null,
  });

export const pollLaputaEvents = (kind: LaputaEventKind, since?: string) =>
  invoke<LaputaEvent[]>("laputa_poll_events", { kind, since: since ?? null });

export interface ListLaputaChangelogFilters {
  section?: LaputaSectionName;
  limit?: number;
  page?: number;
  proposalId?: string;
}

export const listLaputaChangelog = (
  pageOrFilters?: number | ListLaputaChangelogFilters,
  pageSize?: number,
  proposalId?: string,
) => {
  if (typeof pageOrFilters === 'object') {
    const { section, limit, page, proposalId: pid } = pageOrFilters;
    return invoke<ChangelogPage>("laputa_list_changelog", {
      target_section: section ?? null,
      pageSize: limit ?? null,
      page: page ?? null,
      proposalId: pid ?? null,
    });
  }
  return invoke<ChangelogPage>("laputa_list_changelog", {
    page: pageOrFilters ?? null,
    pageSize: pageSize ?? null,
    proposalId: proposalId ?? null,
  });
};

export const getLaputaChangelog = (id: string) =>
  invoke<ChangelogRecord>("laputa_get_changelog", { id });

export const rollbackLaputaChangelog = (id: string, payload: RollbackChangelogPayload) =>
  invoke<RollbackOutcome>("laputa_rollback_changelog", { id, payload });

export const triggerAutoDream = (trigger = 'manual') =>
  invoke<AutoDreamRunRecord>("trigger_autodream", {
    payload: { trigger },
  });

export const getAutoDreamRunStatus = (id: string) =>
  invoke<AutoDreamRunRecord>("get_autodream_run_status", { id });

export const cancelAutoDreamRun = (id: string) =>
  invoke<AutoDreamRunRecord>("cancel_autodream_run", { id });

export const listAutoDreamRunRecords = () =>
  invoke<AutoDreamRunRecord[]>("list_autodream_run_records");

export const getSelfEvolutionConfig = () =>
  invoke<SelfEvolutionConfig>("get_self_evolution_config");

export const saveSelfEvolutionConfig = (config: SelfEvolutionConfig) =>
  invoke<SelfEvolutionConfig>("save_self_evolution_config", { config });

// ============================================================
// Card DTO Interfaces (Story 1.1)
// ============================================================

export interface TodoItem {
  id: string;
  content: string;
  status: 'pending' | 'done';
  completed_at?: string;
}

export interface UiCardAction {
  id: string;
  label: string;
  style: 'primary' | 'secondary' | 'danger' | 'quiet';
  payload: string;
}

export interface UiCard {
  id: string;
  kind: 'decision' | 'todo' | 'approval';
  status: string;
  title: string;
  summary: string;
  body_markdown: string;
  actions: UiCardAction[];
  evidence_refs?: string[];
  risk_level?: 'low' | 'medium' | 'high';
  todo_items?: TodoItem[];
  created_at: string;
  updated_at: string;
}

export interface ApprovalRequest {
  request_id: string;
  operation: string;
  risk: 'low' | 'medium' | 'high';
  scope: string;
  timeout_seconds: number;
  created_at: string;
}

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

// ============================================================
// Sandbox API
// ============================================================

export interface SandboxConfig {
  mode: 'danger-full-access' | 'read-only' | 'workspace-write'
  approval_policy: 'never' | 'on-failure' | 'on-request' | 'unless-trusted'
  network_access: boolean
  writable_roots: string[]
  protected_paths: string[]
  deny_patterns: string[]
  timeout_seconds: number
}

export async function getSandboxConfig(): Promise<SandboxConfig> {
  return invoke<SandboxConfig>('get_sandbox_config')
}

export async function saveSandboxConfig(config: SandboxConfig): Promise<void> {
  return invoke('save_sandbox_config', { config })
}

// ============================================================
// VRM / Desktop GUI Preferences
// ============================================================

export interface GuiPrefs {
  close_to_tray: boolean;
}

export interface MentleToolConfigShape {
  enabled: boolean;
  mode: 'off' | 'read_only' | 'full' | 'custom';
  allowed_tools: string[];
}

export interface MentleToolsListResponse {
  feature_available: boolean;
  tools: string[];
}

export const getGuiPrefs = () =>
  invoke<GuiPrefs>("get_gui_prefs");

export const setGuiPrefs = (prefs: GuiPrefs) =>
  invoke<void>("set_gui_prefs", { prefs });

export const listMentleTools = () =>
  invoke<MentleToolsListResponse>("list_mentle_tools");

// ============================================================
// Mask API
// ============================================================

/** Default settings for subagents spawned under a mask. */
export interface SubagentDefaults {
  model?: string | null;
  max_iterations?: number | null;
}

/** Tool allow/deny limits for a mask. */
export interface ToolLimits {
  allow: string[];
  deny: string[];
}

/** Mask operating mode. */
export type MaskMode = 'normal' | 'assist';

/** Mask entry returned from backend (maps to Rust MaskEntryDto). */
export interface MaskEntryDto {
  name: string;
  icon: string;
  description: string;
  mode: string;
  readOnly: boolean;
}

/** Payload for creating or updating a mask (maps to Rust MaskPayload). */
export interface MaskPayload {
  id?: string | null;
  name: string;
  icon?: string | null;
  description?: string | null;
  mode?: string | null;
  model?: string | null;
  subagentDefaults: SubagentDefaults;
  toolLimits: ToolLimits;
  body?: string | null;
}

export const listMasks = () =>
  invoke<MaskEntryDto[]>('list_masks');

export const getActiveMask = () =>
  invoke<MaskEntryDto | null>('get_active_mask');

export const switchMask = (name: string) =>
  invoke<MaskEntryDto>('switch_mask', { name });

export const createOrUpdateMask = (payload: MaskPayload) =>
  invoke<MaskEntryDto>('create_or_update_mask', { payload });

export const deleteMask = (name: string) =>
  invoke<void>('delete_mask', { name });
