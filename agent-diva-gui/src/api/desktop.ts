import { invoke } from "@tauri-apps/api/core";
import type { PlanApprovalResult, PlanRuntimeState } from "./planning";

export interface GatewayProcessStatus {
  running: boolean;
  pid?: number | null;
  executable_path?: string | null;
  details?: string | null;
}

export interface SkillDto {
  slug: string;
  name: string;
  description: string;
  source: 'builtin' | 'home';
  enabled: boolean;
  always: boolean;
  available: boolean;
  active: boolean;
  content_hash: string;
  updated_at: string;
  can_hard_delete: boolean;
  path: string;
  can_delete: boolean;
}

export interface SkillDocument extends Omit<SkillDto, 'name' | 'active' | 'path' | 'can_delete'> {
  markdown: string;
}

export interface SkillWriteOutcome {
  document: SkillDocument;
  changed: boolean;
}

export interface SkillHistoryEntry {
  revision: number;
  content_hash: string;
  updated_at: string;
}

export interface SkillHistoryDocument {
  slug: string;
  revision: number;
  content_hash: string;
  markdown: string;
}

export interface SkillEvidence {
  session_key?: string | null;
  actmem_pointer?: string | null;
  autodream_run_id?: string | null;
  tool?: string | null;
  artifact?: string | null;
}

export type SkillRequestStatus = 'pending' | 'accepted' | 'rejected' | 'stale';
export type SkillRequestSource = 'autodream' | 'distill' | 'user_request';

export interface SkillRequest {
  id: string;
  slug: string;
  title: string;
  proposed_markdown: string;
  evidence: SkillEvidence[];
  attestation?: string | null;
  base_hash: string;
  source: SkillRequestSource;
  reason: string;
  status: SkillRequestStatus;
  created_at: string;
  updated_at: string;
}

export interface CreateSkillRequestPayload {
  slug: string;
  title: string;
  proposed_markdown: string;
  evidence?: SkillEvidence[];
  attestation?: string | null;
  base_hash: string;
  reason: string;
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

export interface PlanApprovalRequest {
  session_key: string;
  plan_id?: string;
  expected_revision: number;
  markdown: string;
  todo_policy: 'Never' | 'Optional' | 'Always';
  materialize_todos: boolean;
  context_policy?: 'retain' | 'compact' | 'clear';
}

export const approveActivePlanExecution = (request: PlanApprovalRequest) =>
  invoke<PlanApprovalResult>("approve_active_plan_execution", { request });

export const deletePlan = (planId: string) =>
  invoke<void>("delete_plan", { planId });

export const returnActivePlanToDraft = (sessionKey?: string) =>
  invoke<PlanRuntimeState>("return_active_plan_to_draft", { sessionKey });

export const saveRawConfig = (raw: string) =>
  invoke<void>("save_config", { raw });

export interface GuiLogEntry {
  timestamp: string;
  level: string;
  source: 'gui';
  message: string;
  args: unknown;
  windowLabel: string;
}

export const appendGuiLog = (entries: GuiLogEntry[]) =>
  invoke<void>('append_gui_log', { entries });

export const getGatewayLogLines = (date: string, maxLines = 500) =>
  invoke<string[]>('get_gateway_log_lines', { date, maxLines });

export const getGuiLogLines = (date: string, maxLines = 500) =>
  invoke<string[]>('get_gui_log_lines', { date, maxLines });

export const checkHealth = () => invoke<boolean>("check_health");

export const getSkills = () => invoke<SkillDto[]>("get_skills");

export const getSkill = (slug: string) =>
  invoke<SkillDocument>('get_skill', { slug });

export const updateSkill = (slug: string, markdown: string, baseHash: string) =>
  invoke<SkillWriteOutcome>('update_skill', { slug, markdown, baseHash });

export const disableSkill = (slug: string, baseHash: string) =>
  invoke<SkillWriteOutcome>('disable_skill', { slug, baseHash });

export const listSkillHistory = (slug: string) =>
  invoke<SkillHistoryEntry[]>('list_skill_history', { slug });

export const getSkillHistoryRevision = (slug: string, revision: number) =>
  invoke<SkillHistoryDocument>('get_skill_history_revision', { slug, revision });

export const listSkillRequests = () =>
  invoke<SkillRequest[]>('list_skill_requests');

export const createSkillRequest = (payload: CreateSkillRequestPayload) =>
  invoke<SkillRequest>('create_skill_request', { payload });

export const getSkillRequest = (id: string) =>
  invoke<SkillRequest>('get_skill_request', { id });

export const acceptSkillRequest = (id: string) =>
  invoke<SkillRequest>('accept_skill_request', { id });

export const rejectSkillRequest = (id: string) =>
  invoke<SkillRequest>('reject_skill_request', { id });

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

export const deleteSkill = (slug: string, baseHash: string) =>
  invoke<void>("delete_skill", { slug, baseHash });

// ============================================================
// AutoDream / Evolution API
// ============================================================

export type AutoDreamRunState = 'pending' | 'running' | 'cancelled' | 'completed' | 'failed';
export type AutoDreamFailureCode =
  | 'cancelled'
  | 'input_unavailable'
  | 'worker_timeout'
  | 'worker_failed'
  | 'report_generation_failed'
  | 'stale_run_recovered';

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
  orchestration?: {
    schema_version: number;
    phase:
      | 'queued'
      | 'gathering'
      | 'reflecting'
      | 'validating'
      | 'publishing'
      | 'completed'
      | 'failed'
      | 'cancelled';
    attempt: number;
    deadline_at: string;
    updated_at: string;
  } | null;
  failure_code?: AutoDreamFailureCode | null;
  error?: string | null;
}

export interface AutoDreamRunEvent {
  id: string;
  run_id?: string | null;
  kind: string;
  message: string;
  created_at: string;
}

export interface RecallFeedbackEvent {
  schema_version: number;
  event_id: string;
  request_id: string;
  record_id: string;
  content_digest: { algorithm: string; value: string };
  selected: boolean;
  injected: boolean;
  corrected: boolean;
  task_outcome: 'succeeded' | 'failed' | 'unknown';
  recorded_at: string;
}

export interface SelfEvolutionConfig {
  enabled: boolean;
  autodream_frequency: 'daily' | 'weekly' | 'manual';
  trigger_threshold_sessions: number;
  trigger_threshold_messages: number;
  auto_merge_confidence: number;
  require_confirmation_for: string[];
}

export type PersonaKind = 'identity' | 'relationship' | 'redline' | 'user' | 'world' | 'dream' | 'dark';
export type PersonaStatus = 'uninitialized' | 'ready' | 'incomplete';

export interface PersonaFileState {
  kind: PersonaKind;
  file_name: string;
  exists: boolean;
  valid: boolean;
  reason?: string | null;
  revision: number;
  updated_at?: string | null;
  pending_count: number;
}

export interface PersonaStatusView {
  status: PersonaStatus;
  files: Record<PersonaKind, PersonaFileState>;
}

export interface PersonaDocument {
  kind: PersonaKind;
  file_name: string;
  exists: boolean;
  valid: boolean;
  content: string;
  revision: number;
  content_hash: string;
  updated_at?: string | null;
  pending_count: number;
}

export type PersonaRequestState = 'pending' | 'accepted' | 'rejected' | 'stale';

export interface PersonaChangeRequest {
  id: string;
  kind: PersonaKind;
  base_revision: number;
  base_hash: string;
  proposed_markdown: string;
  actor: 'agent' | 'autodream';
  reason: string;
  created_at: string;
  state: PersonaRequestState;
  decided_at?: string | null;
}

export interface PersonaHistoryEntry {
  revision: number;
  content_hash: string;
  snapshot: string;
  diff: string;
  actor: string;
  source: string;
  reason: string;
  base_revision: number;
  created_at: string;
}

export interface PersonaHistoryRevision extends PersonaHistoryEntry {
  content: string;
  unified_diff: string;
}

export interface PersonaInitializationPayload {
  identity: string;
  relationship: string;
  redline: string;
  user: string;
  world: string;
}

export const getPersonaStatus = () => invoke<PersonaStatusView>('persona_get_status');
export const initializePersona = (payload: PersonaInitializationPayload) =>
  invoke<PersonaStatusView>('persona_initialize', { payload });
export const repairPersona = (documents: Partial<Record<PersonaKind, string>>) =>
  invoke<PersonaStatusView>('persona_repair', { payload: { documents } });
export const getPersonaDocument = (kind: PersonaKind) =>
  invoke<PersonaDocument>('persona_get_document', { kind });
export const savePersonaDocument = (kind: PersonaKind, content: string, baseRevision: number, reason: string) =>
  invoke<{ document: PersonaDocument; changed: boolean }>('persona_save_document', {
    kind,
    payload: { content, base_revision: baseRevision, reason },
  });
export const listPersonaHistory = (kind: PersonaKind) =>
  invoke<PersonaHistoryEntry[]>('persona_list_history', { kind });
export const getPersonaHistoryRevision = (kind: PersonaKind, revision: number) =>
  invoke<PersonaHistoryRevision>('persona_get_history_revision', { kind, revision });
export const listPersonaRequests = (kind?: PersonaKind) =>
  invoke<PersonaChangeRequest[]>('persona_list_requests', { kind: kind ?? null });
export const acceptPersonaRequest = (id: string) =>
  invoke<PersonaChangeRequest>('persona_accept_request', { id });
export const rejectPersonaRequest = (id: string) =>
  invoke<PersonaChangeRequest>('persona_reject_request', { id });

export type EvidenceSource =
  | 'session'
  | 'report'
  | 'auto_dream_run'
  | 'user_input'
  | 'file'
  | 'context_compaction'
  | 'experience_journal'
  | 'recall_feedback';

export interface EvidenceRef {
  id: string;
  source: EvidenceSource;
  uri: string;
  excerpt?: string | null;
  hash?: string | null;
  created_at: string;
}

export interface MemoryRecord {
  id: string;
  content: string;
  trust: string;
  provenance?: string | null;
  evidence_refs: EvidenceRef[];
  revision: number;
  created_at: string;
  updated_at: string;
}

export interface ActmemDocument {
  revision: number;
  updated_at: string;
  pulse: string;
  recap: string;
  work: string;
  markdown: string;
}

export interface ActmemCapsuleSummary {
  name: string;
  session_key: string;
  created_at: string;
  chars: number;
}

export interface ActmemCapsule {
  name: string;
  session_key: string;
  created_at: string;
  markdown: string;
}

export interface MemoryRulesDocument {
  content: string;
  source: 'default' | 'file';
}

export const listMemoryRecords = (limit = 100) =>
  invoke<MemoryRecord[]>('memory_list_records', { limit });

export const createMemoryRecord = (content: string) =>
  invoke<MemoryRecord>('memory_create_record', { payload: { content } });

export const getMemoryRecord = (id: string) =>
  invoke<MemoryRecord>('memory_get_record', { id });

export const updateMemoryRecord = (id: string, content: string, baseRevision: number) =>
  invoke<MemoryRecord>('memory_update_record', {
    id,
    payload: { content, base_revision: baseRevision },
  });

export const deleteMemoryRecord = (id: string, reason: string, baseRevision: number) =>
  invoke<{ record: MemoryRecord; deleted: boolean }>('memory_delete_record', {
    id,
    payload: { reason, base_revision: baseRevision },
  });

export const getActmem = () => invoke<ActmemDocument>('memory_get_actmem');

export const putActmem = (payload: {
  pulse?: string;
  recap?: string;
  work?: string;
  base_revision: number;
}) => invoke<ActmemDocument>('memory_put_actmem', { payload });

export const listActmemCapsules = () =>
  invoke<ActmemCapsuleSummary[]>('memory_list_capsules');

export const getActmemCapsule = (name: string) =>
  invoke<ActmemCapsule>('memory_get_capsule', { name });

export const deleteActmemCapsule = (name: string) =>
  invoke<{ deleted: boolean }>('memory_delete_capsule', { name });

export const getMemoryRules = () =>
  invoke<MemoryRulesDocument>('memory_get_memrules');

export const putMemoryRules = (content: string) =>
  invoke<MemoryRulesDocument>('memory_put_memrules', { payload: { content } });

export const triggerAutoDream = (trigger = 'manual') =>
  invoke<AutoDreamRunRecord>("trigger_autodream", {
    payload: { trigger },
  });

export const getAutoDreamRunStatus = (id: string) =>
  invoke<AutoDreamRunRecord>("get_autodream_run_status", { id });

export const listAutoDreamRunEvents = (id: string) =>
  invoke<AutoDreamRunEvent[]>("list_autodream_run_events", { id });

export const getAutoDreamLiveText = (id: string) =>
  invoke<string>("get_autodream_live_text", { id });

export const cancelAutoDreamRun = (id: string) =>
  invoke<AutoDreamRunRecord>("cancel_autodream_run", { id });

export const listAutoDreamRunRecords = () =>
  invoke<AutoDreamRunRecord[]>("list_autodream_run_records");

export const listRecallFeedback = (limit = 50) =>
  invoke<RecallFeedbackEvent[]>("list_recall_feedback", { limit });

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

export interface ChecklistItem {
  step: string;
  status:
    | 'pending'
    | 'in_progress'
    | 'completed'
    | 'Pending'
    | 'InProgress'
    | 'Completed';
}

export interface ChecklistCard {
  id: string;
  kind: 'checklist';
  explanation?: string;
  plan: ChecklistItem[];
  created_at: string;
  updated_at: string;
}

export interface UiCardAction {
  id: string;
  label: string;
  style: 'primary' | 'secondary' | 'danger' | 'quiet';
  payload: string;
}

export interface UiCard {
  id: string;
  kind: 'decision' | 'todo' | 'approval' | 'checklist';
  status: string;
  title: string;
  summary: string;
  body_markdown: string;
  actions: UiCardAction[];
  evidence_refs?: string[];
  risk_level?: 'low' | 'medium' | 'high';
  todo_items?: TodoItem[];
  plan_items?: ChecklistItem[];
  explanation?: string;
  created_at: string;
  updated_at: string;
}

export interface CommandApprovalScope {
  channel: string;
  chat_id: string;
  session_key: string;
}

export type ApprovalDecision = 'approve_once' | 'approve_session' | 'approve_global' | 'reject';

export interface SafePrefixSuggestion {
  pattern: string[];
  justification: string;
}

export interface CommandApprovalRequest {
  approval_id: string;
  command: string;
  cwd: string;
  reason: string;
  scope: CommandApprovalScope;
  timeout_seconds: number;
  created_at: string;
  suggested_prefix?: SafePrefixSuggestion;
}

export interface CommandApprovalResolution {
  approval_id: string;
  decision: ApprovalDecision;
  status: 'approved' | 'rejected' | 'expired' | 'cancelled';
}

export interface CommandRule {
  id: string;
  pattern: string[];
  decision: string;
  enabled: boolean;
  source: 'legacy' | 'approval';
  justification: string;
  created_at: string;
  revision: number;
}

export async function getCommandRules(): Promise<CommandRule[]> {
  return invoke<CommandRule[]>('get_command_rules');
}

export async function setCommandRuleEnabled(
  rule: Pick<CommandRule, 'id' | 'revision'>,
  enabled: boolean,
): Promise<CommandRule> {
  return invoke<CommandRule>('set_command_rule_enabled', {
    ruleId: rule.id,
    revision: rule.revision,
    enabled,
  });
}

export async function deleteCommandRule(
  rule: Pick<CommandRule, 'id' | 'revision'>,
): Promise<void> {
  return invoke('delete_command_rule', { ruleId: rule.id, revision: rule.revision });
}

// ============================================================
// Marketplace API (skills.sh via gateway adapter)
// ============================================================

export interface MarketplaceSkillEntry {
  /** Fully qualified id: owner/repo/slug. */
  id: string;
  /** Skill name as listed by the directory. */
  name: string;
  /** Source repository: owner/repo. */
  source: string;
  /** Reported install count. */
  installs: number;
}

export async function searchMarketplaceSkills(
  query: string,
  limit?: number
): Promise<MarketplaceSkillEntry[]> {
  return invoke<MarketplaceSkillEntry[]>("search_marketplace_skills", {
    query,
    limit: limit ?? null,
  });
}

export async function installMarketplaceSkill(id: string): Promise<SkillDto> {
  return invoke<SkillDto>("install_marketplace_skill", { id });
}

export interface MarketplaceFeaturedResponse {
  skills: MarketplaceSkillEntry[];
  total?: number;
  generated_at?: string;
  source?: string;
  metric?: string;
}

export async function featuredMarketplaceSkills(): Promise<MarketplaceFeaturedResponse> {
  return invoke<MarketplaceFeaturedResponse>("featured_marketplace_skills");
}

// ============================================================
// Sandbox API
// ============================================================

export interface SandboxConfig {
  mode: 'danger_full_access' | 'read_only' | 'workspace_write'
  approval_policy: 'never' | 'on_failure' | 'on_request' | 'unless_trusted'
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

export const getGuiPrefs = () =>
  invoke<GuiPrefs>("get_gui_prefs");

export const setGuiPrefs = (prefs: GuiPrefs) =>
  invoke<void>("set_gui_prefs", { prefs });

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
