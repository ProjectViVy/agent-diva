<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import type { AskUserQuestionView, CompactionStatus } from './components/ChatView.vue';
import NormalMode from "./components/NormalMode.vue";
import ApprovalCenterDrawer from "./components/ApprovalCenterDrawer.vue";
import WelcomeWizard from "./components/WelcomeWizard.vue";
import PersonaSetupGate from "./components/PersonaSetupGate.vue";
import { appAlert, appConfirm } from "./utils/appDialog";
import { showAppToast } from "./utils/appToast";
import { useI18n } from "vue-i18n";
import {
  approveActivePlanExecution,
  getConfigStatus,
  getRuntimeConfig,
  returnActivePlanToDraft,
  switchWorkspace as switchWorkspaceApi,
  FileAttachmentDto,
  ChecklistItem,
  type WorkspaceSwitchRequest,
} from "./api/desktop";
import {
  planReportValidationIssues,
  type PlanDetail,
  type PlanRuntimeState,
  type PlanStreamEvent,
} from "./api/planning";
import type { ToolsConfigShape } from "./types/toolsConfig";
import { useWorkspaceContext } from "./composables/useWorkspaceContext";
import {
  ApprovalEventGuard,
  isApprovalEventView,
  isApprovalListPage,
  isApprovalView,
  type ApprovalEventView,
  type ApprovalGrant,
  type ApprovalListPage,
  type ApprovalView,
  type UnifiedApprovalApiError,
} from "./api/approvals";
import {
  HISTORY_PREFS_KEY,
  SAVED_MODELS_KEY,
  SESSION_CACHE_PREFIX,
  WELCOME_STORAGE_KEY,
} from "./utils/localStorageAgentDiva";
import {
  DEFAULT_DEEPSEEK_API_BASE,
  DEFAULT_DEEPSEEK_MODEL,
  DEFAULT_DEEPSEEK_PROVIDER,
  buildWelcomeDeepSeekConfig,
} from "./utils/welcomeConfig";
import {
  completeLatestStreamingAgent,
  findCurrentTurnUpdatePlanToolIndex,
  findLatestStreamingAgentIndex,
} from "./utils/streamingMessages";

const { t } = useI18n();
const {
  status: workspace,
  state: workspaceState,
  error: workspaceError,
  refresh: refreshWorkspace,
  apply: applyWorkspaceStatus,
} = useWorkspaceContext();

type ExecMode = 'agent' | 'plan' | 'ask';

interface Message {
  id: string;
  role: 'user' | 'agent' | 'system' | 'tool';
  content: string;
  reasoning?: string;
  isThinking?: boolean;
  isStreaming?: boolean;
  timestamp?: number;
  emotion?: string;
  toolName?: string;
  toolArgs?: string;
  toolResult?: string;
  toolStatus?: 'running' | 'success' | 'error';
  toolCallId?: string;
  retryStatus?: { attempt: number; maxRetries: number; model?: string };
  stalled?: boolean;
  rawMeta?: Record<string, unknown>;
  fromHistory?: boolean;
  attachments?: string[];
}

interface ToolStartPayload {
  name: string;
  args_preview?: string;
  call_id?: string | null;
}

interface ToolFinishPayload {
  name: string;
  result: string;
  is_error?: boolean;
  call_id?: string | null;
}

interface StreamTextPayload {
  request_id: string;
  data: string;
}

interface StreamRetryPayload {
  request_id: string;
  model: string;
  attempt: number;
  max_retries: number;
  delay_ms: number;
  reason: string;
}

interface StreamStalledPayload {
  request_id: string;
  model?: string | null;
}

interface StreamContextCompactionPayload {
  request_id: string;
  data: {
    session_id: string;
    trigger: string;
    phase: string;
    summary?: string | null;
  };
}

interface StreamToolStartPayload extends ToolStartPayload {
  request_id: string;
}

interface StreamToolFinishPayload extends ToolFinishPayload {
  request_id: string;
}

interface StreamPlanPayload {
  request_id: string;
  data: PlanStreamEvent;
}

interface StreamJsonPayload {
  request_id: string;
  data: unknown;
}

interface StreamTurnPlanPayload {
  request_id: string;
  data: {
    explanation?: string;
    plan: ChecklistItem[];
  };
}

interface SavedModel {
  id: string;
  provider: string;
  model: string;
  apiBase: string;
  apiKey: string;
  displayName: string;
}

interface SessionInfo {
  session_key: string;
  chat_id: string;
  snippet: string;
  timestamp: number;
  title?: string;
  last_message?: string;
  message_count: number;
  title_generated: boolean;
  title_manually_set: boolean;
  pinned?: boolean;
  workspace_id?: string;
  channel?: string;
  kind?: 'root' | 'branch' | 'subagent' | 'ephemeral';
  root_session_key?: string | null;
  parent_session_key?: string | null;
  branch_label?: string | null;
  legacy?: boolean;
}
interface ChatDisplayPrefs {
  cleanMode: boolean;
  autoExpandReasoning: boolean;
  autoExpandToolDetails: boolean;
  showRawMetaByDefault: boolean;
}

interface BackendSessionInfo {
  key: string;
  created_at?: string | null;
  updated_at?: string | null;
  path?: string;
  title?: string;
  last_message?: string | null;
  message_count?: number;
  title_generated?: boolean;
  title_manually_set?: boolean;
  pinned?: boolean;
  workspace_id?: string;
  channel?: string;
  kind?: 'root' | 'branch' | 'subagent' | 'ephemeral';
  root_session_key?: string | null;
  parent_session_key?: string | null;
  branch_label?: string | null;
  legacy?: boolean;
}

interface BackendChatMessage {
  role: string;
  content: string;
  timestamp?: string;
  reasoning_content?: string | null;
  tool_call_id?: string | null;
  tool_calls?: serdeJsonValue[] | null;
  name?: string | null;
  thinking_blocks?: serdeJsonValue[] | null;
  metadata?: serdeJsonValue | null;
}

interface BackendSessionHistory {
  key: string;
  messages: BackendChatMessage[];
}

type serdeJsonValue = Record<string, unknown>;

interface SessionCacheEntry {
  session: BackendSessionHistory;
  cachedAt: number;
}

interface RawProviderConfig {
  api_key?: string;
  api_base?: string | null;
}

interface RawConfigShape {
  agents?: { defaults?: { provider?: string | null; model?: string | null } };
  providers?: Record<string, RawProviderConfig> & {
    custom_providers?: Record<string, RawProviderConfig>;
  };
}

interface ProviderConfigEntry {
  apiKey: string;
  apiBase: string;
  source: 'providers' | 'custom_providers';
}

const SESSION_CACHE_TTL_MS = 30 * 60 * 1000;
const STARTUP_TASK_TIMEOUT_MS = 2500;
const SESSION_LOAD_TIMEOUT_MS = 2000;
const defaultChatDisplayPrefs: ChatDisplayPrefs = {
  cleanMode: false,
  autoExpandReasoning: true,
  autoExpandToolDetails: false,
  showRawMetaByDefault: false,
};

const messages = ref<Message[]>([
  {
    id: generateChatId(),
    role: 'agent',
    content: t('app.welcome'),
    timestamp: Date.now(),
    emotion: 'happy'
  }
]);
const isTyping = ref(false);
const connectionStatus = ref<'connected' | 'error' | 'connecting'>('connected');
const currentEmotion = ref('happy');
const suppressNextStopError = ref(false);
const currentChannel = ref('gui');
const currentChatId = ref(generateChatId());
const currentSessionKey = ref(`gui:${currentChatId.value}`);
const compactionStatus = ref<CompactionStatus | null>(null);
const activeStreamRequestId = ref<string | null>(null);
const workspaceSwitching = ref(false);
const activePlanRuntime = ref<PlanRuntimeState | null>(null);
const pendingApprovalPlan = ref<PlanRuntimeState | null>(null);
const pendingApprovalSessionKey = ref<string | null>(null);
const executingPlan = ref<PlanRuntimeState | null>(null);
const approvingPlan = ref(false);
type PlanContinuationContext = {
  planId: string;
  revision?: number;
  executionId?: string | null;
  sessionKey: string;
};
const planContinuationContext = ref<PlanContinuationContext | null>(null);
const planContinuationError = ref<string | null>(null);
const locallyDeletedSessionKeys = ref<Set<string>>(new Set());
const titleGenerationInFlight = ref<Set<string>>(new Set());

const currentPlanContinuationError = computed(() =>
  planContinuationContext.value?.sessionKey === currentSessionKey.value
    ? planContinuationError.value
    : null,
);

watch(currentSessionKey, () => {
  compactionStatus.value = null;
});

// Config state
const config = ref({
  provider: DEFAULT_DEEPSEEK_PROVIDER,
  apiBase: DEFAULT_DEEPSEEK_API_BASE,
  apiKey: "",
  model: DEFAULT_DEEPSEEK_MODEL
});

const toolsConfig = ref<ToolsConfigShape>({
  web: {
    search: {
      provider: 'bocha',
      enabled: true,
      api_key: '',
      max_results: 5
    },
    fetch: {
      enabled: true
    }
  },
  budget: {
    max_tokens: 180000,
    system_budget_ratio: 0.15,
    compact_threshold_ratio: 0.8,
    keep_recent_count: 10,
  },
});

const savedModels = ref<SavedModel[]>([]);
const providerConfigs = ref<Record<string, ProviderConfigEntry>>({});
const sessions = ref<SessionInfo[]>([]);
const chatDisplayPrefs = ref<ChatDisplayPrefs>({ ...defaultChatDisplayPrefs });
const approvalCenterOpen = ref(false);
const approvalDrawerAutoOpened = ref(false);
const unifiedApprovals = ref<ApprovalView[]>([]);
const approvalDetails = ref<Record<string, ApprovalView>>({});
const approvalCenterLoading = ref(false);
const approvalCenterError = ref<string | null>(null);
const unifiedSubmittingIds = ref<string[]>([]);
const unifiedOutcomeUnknownIds = ref<string[]>([]);
const unifiedActionErrors = ref<Record<string, string>>({});
const approvalEventGuard = new ApprovalEventGuard();
const approvalVersions = new Map<string, number>();

const unlisteners: UnlistenFn[] = [];

const showWelcomeWizard = ref(false);
const normalModeRef = ref<InstanceType<typeof NormalMode> | null>(null);

const approvalPendingCount = computed(() =>
  unifiedApprovals.value.filter((approval) => approval.status === 'pending').length,
);

const pendingQuestions = ref<AskUserQuestionView[]>([]);
const askUserSubmittingIds = ref<string[]>([]);
const askUserError = ref<string | null>(null);
let askUserPollTimer: ReturnType<typeof setInterval> | null = null;

const workspaceSwitchBlockedReason = computed(() => {
  if (workspaceSwitching.value) return '工作区切换进行中，请稍候。';
  if (isTyping.value) return '当前仍有流式输出，请先停止并等待会话保存。';
  if (activePlanRuntime.value || pendingApprovalPlan.value || executingPlan.value || approvingPlan.value) {
    return '当前有未结束的 Plan 或审批流程，请先完成或取消。';
  }
  if (approvalPendingCount.value > 0) return '当前有待处理审批，请先处理完审批。';
  if (pendingQuestions.value.length > 0) return '当前有等待用户回答的 HITL 问题，请先回答或取消。';
  return null;
});

async function listAskUserQuestions() {
  if (!isTauri()) return;
  try {
    const response = await invoke<{ questions: AskUserQuestionView[] }>('list_ask_user_questions');
    pendingQuestions.value = Array.isArray(response?.questions) ? response.questions : [];
  } catch (error) {
    console.warn('list_ask_user_questions failed:', error);
  }
}

async function answerAskUserQuestion(payload: { question_id: string; selected_index: number | null; other_text: string | null }) {
  if (!isTauri()) return;
  askUserSubmittingIds.value = [...askUserSubmittingIds.value, payload.question_id];
  askUserError.value = null;
  try {
    await invoke('answer_ask_user_question', {
      questionId: payload.question_id,
      selectedIndex: payload.selected_index,
      otherText: payload.other_text,
    });
    await listAskUserQuestions();
  } catch (error) {
    askUserError.value = String(error);
  } finally {
    askUserSubmittingIds.value = askUserSubmittingIds.value.filter((id) => id !== payload.question_id);
  }
}

async function cancelAskUserQuestion(questionId: string) {
  if (!isTauri()) return;
  askUserSubmittingIds.value = [...askUserSubmittingIds.value, questionId];
  askUserError.value = null;
  try {
    await invoke('cancel_ask_user_question', { questionId });
    await listAskUserQuestions();
  } catch (error) {
    askUserError.value = String(error);
  } finally {
    askUserSubmittingIds.value = askUserSubmittingIds.value.filter((id) => id !== questionId);
  }
}

function upsertUnifiedApproval(approval: ApprovalView) {
  const current = unifiedApprovals.value.find((item) => item.request_id === approval.request_id);
  if (current && current.version > approval.version) return;
  approvalVersions.set(approval.request_id, approval.version);
  unifiedApprovals.value = [
    ...unifiedApprovals.value.filter((item) => item.request_id !== approval.request_id),
    approval,
  ];
}

function unifiedError(error: unknown): UnifiedApprovalApiError | null {
  if (!error || typeof error !== 'object') return null;
  const value = error as Partial<UnifiedApprovalApiError>;
  return typeof value.reason_code === 'string' && typeof value.status === 'number'
    ? value as UnifiedApprovalApiError
    : null;
}

function approvalActionMessage(error: unknown): string {
  const typed = unifiedError(error);
  if (typed?.reason_code === 'approval_outcome_unknown') return t('approvalCenter.outcomeUnknown');
  if (typed?.status === 409) return t('approvalCenter.stale');
  if (typed?.message) return typed.message;
  if (error instanceof Error && error.message) return error.message;
  if (typeof error === 'string' && error.trim()) return error;
  return t('approvalCenter.requestFailed');
}

async function refreshUnifiedApprovals() {
  if (!isTauri() || approvalCenterLoading.value) return;
  approvalCenterLoading.value = true;
  approvalCenterError.value = null;
  try {
    const approvals: ApprovalView[] = [];
    let cursor: string | null = null;
    for (let pageIndex = 0; pageIndex < 10; pageIndex += 1) {
      const raw: ApprovalListPage = await invoke<ApprovalListPage>('list_approvals', {
        domain: null,
        status: null,
        session: null,
        cursor,
        limit: 100,
      });
      if (!isApprovalListPage(raw)) throw new Error('invalid approval list response');
      approvals.push(...raw.approvals);
      cursor = raw.next_cursor;
      if (!cursor) break;
      if (pageIndex === 9) approvalCenterError.value = t('approvalCenter.pageLimit');
    }
    unifiedApprovals.value = approvals;
    for (const approval of approvals) approvalVersions.set(approval.request_id, approval.version);
    const nearestPending = approvals
      .filter((approval) => approval.status === 'pending')
      .sort((left, right) => Date.parse(left.expires_at) - Date.parse(right.expires_at))
      .slice(0, 25);
    await Promise.allSettled(nearestPending.map((approval) => refreshUnifiedApproval(approval.request_id)));
  } catch (error) {
    approvalCenterError.value = approvalActionMessage(error);
  } finally {
    approvalCenterLoading.value = false;
  }
}

async function refreshUnifiedApproval(requestId: string) {
  if (!isTauri()) return;
  try {
    const detail = await invoke<ApprovalView>('get_approval', { requestId });
    if (!isApprovalView(detail)) throw new Error('invalid approval detail response');
    approvalDetails.value = { ...approvalDetails.value, [requestId]: detail };
    upsertUnifiedApproval(detail);
    unifiedOutcomeUnknownIds.value = unifiedOutcomeUnknownIds.value.filter((id) => id !== requestId);
    const errors = { ...unifiedActionErrors.value };
    delete errors[requestId];
    unifiedActionErrors.value = errors;
  } catch (error) {
    unifiedActionErrors.value = { ...unifiedActionErrors.value, [requestId]: approvalActionMessage(error) };
  }
}

function approvalIdempotencyKey(requestId: string, operation: string): string {
  const random = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  return `gui-${operation}-${requestId}-${random}`;
}

async function prepareUnifiedPlanApproval(approval: ApprovalView): Promise<PlanApprovalTarget> {
  if (approval.status !== 'pending') {
    throw new Error('This plan approval is no longer pending; refresh the approval center.');
  }
  const presentationSession = approval.presentation?.session_key;
  const sessionKey = approval.resource.session_id
    || (typeof presentationSession === 'string' ? presentationSession.trim() : '');
  if (!sessionKey) throw new Error('The plan approval has no source session.');

  const rawReports = await invoke<unknown>('get_plan_reports');
  if (!Array.isArray(rawReports)) throw new Error('Invalid plan report response.');
  const rawReport = rawReports.find((report) => planReportId(report) === approval.resource.resource_id);
  if (!rawReport) throw new Error('The approved plan report is no longer available.');
  const reportSession = planReportSessionKey(rawReport);
  if (reportSession && reportSession !== sessionKey) {
    throw new Error('The approval source session no longer matches the plan report.');
  }
  const plan = planRuntimeFromReportPayload(rawReport);
  if (!plan || plan.plan_id !== approval.resource.resource_id) {
    throw new Error('The approval plan payload is unavailable; refresh and retry.');
  }
  if (isTyping.value) throw new Error('Another response is already streaming.');
  if (currentSessionKey.value !== sessionKey) {
    const loaded = await loadSession(sessionKey);
    if (!loaded) throw new Error('The plan source session could not be loaded.');
  }
  return { plan, sessionKey };
}

async function decideUnifiedApproval(payload: { approval: ApprovalView; decision: 'allow' | 'deny'; grant: ApprovalGrant }) {
  const { approval } = payload;
  if (unifiedSubmittingIds.value.includes(approval.request_id)) return;
  unifiedSubmittingIds.value = [...unifiedSubmittingIds.value, approval.request_id];
  const clearActionError = () => {
    const errors = { ...unifiedActionErrors.value };
    delete errors[approval.request_id];
    unifiedActionErrors.value = errors;
  };
  clearActionError();
  try {
    if (approval.domain === 'plan' && payload.decision === 'allow') {
      if (payload.grant !== 'once') throw new Error('Plan approvals only support a one-time grant.');
      const target = await prepareUnifiedPlanApproval(approval);
      await runPlanApproval(
        {
          contextPolicy: 'retain',
          todoPolicy: 'Optional',
          materializeTodos: false,
        },
        target,
      );
      await refreshUnifiedApproval(approval.request_id);
      approvalDrawerAutoOpened.value = false;
      approvalCenterOpen.value = false;
      return;
    }
    const result = await invoke<ApprovalView>('decide_approval', {
      requestId: approval.request_id,
      payload: {
        expected_version: approval.version,
        idempotency_key: approvalIdempotencyKey(approval.request_id, payload.decision),
        decision: payload.decision,
        grant: payload.grant,
      },
    });
    if (!isApprovalView(result)) throw new Error('invalid approval decision response');
    upsertUnifiedApproval(result);
    approvalDetails.value = { ...approvalDetails.value, [approval.request_id]: result };
    if (payload.decision === 'allow' && approvalDrawerAutoOpened.value) {
      approvalDrawerAutoOpened.value = false;
      approvalCenterOpen.value = false;
    }
  } catch (error) {
    const typed = unifiedError(error);
    if (typed?.reason_code === 'approval_outcome_unknown') {
      unifiedOutcomeUnknownIds.value = [...new Set([...unifiedOutcomeUnknownIds.value, approval.request_id])];
    }
    // A plan may already be durably approved when stream startup fails. Fetch
    // the authoritative state before rendering the recoverable error.
    if (approval.domain === 'plan' && payload.decision === 'allow') {
      await refreshUnifiedApproval(approval.request_id);
      if (planContinuationError.value) {
        // The consumed approval is filtered out of the pending drawer. Close
        // it so the source chat's retry affordance remains reachable.
        approvalDrawerAutoOpened.value = false;
        approvalCenterOpen.value = false;
      }
    }
    unifiedActionErrors.value = { ...unifiedActionErrors.value, [approval.request_id]: approvalActionMessage(error) };
    if (typed?.status === 409 || typed?.status === 404) await refreshUnifiedApproval(approval.request_id);
  } finally {
    unifiedSubmittingIds.value = unifiedSubmittingIds.value.filter((id) => id !== approval.request_id);
  }
}

async function cancelUnifiedApproval(approval: ApprovalView) {
  if (unifiedSubmittingIds.value.includes(approval.request_id)) return;
  unifiedSubmittingIds.value = [...unifiedSubmittingIds.value, approval.request_id];
  try {
    const result = await invoke<ApprovalView>('cancel_approval', {
      requestId: approval.request_id,
      payload: {
        expected_version: approval.version,
        idempotency_key: approvalIdempotencyKey(approval.request_id, 'cancel'),
      },
    });
    if (!isApprovalView(result)) throw new Error('invalid approval cancellation response');
    upsertUnifiedApproval(result);
    approvalDetails.value = { ...approvalDetails.value, [approval.request_id]: result };
  } catch (error) {
    const typed = unifiedError(error);
    if (typed?.reason_code === 'approval_outcome_unknown') {
      unifiedOutcomeUnknownIds.value = [...new Set([...unifiedOutcomeUnknownIds.value, approval.request_id])];
    }
    unifiedActionErrors.value = { ...unifiedActionErrors.value, [approval.request_id]: approvalActionMessage(error) };
  } finally {
    unifiedSubmittingIds.value = unifiedSubmittingIds.value.filter((id) => id !== approval.request_id);
  }
}

function editUnifiedApproval(approval: ApprovalView) {
  approvalCenterOpen.value = false;
  approvalDrawerAutoOpened.value = false;
  if (approval.resource.session_id) void loadSession(approval.resource.session_id);
  showAppToast(t('approvalCenter.editAtSource'));
}

function onApprovalCenterOpenChange(open: boolean) {
  approvalCenterOpen.value = open;
  if (open) approvalDrawerAutoOpened.value = false;
}

async function handleUnifiedApprovalEvent(payload: ApprovalEventView) {
  const knownVersion = approvalVersions.get(payload.request_id) ?? 0;
  if (!approvalEventGuard.accept(payload, knownVersion)) return;
  if (payload.status === 'pending') {
    approvalCenterOpen.value = true;
    approvalDrawerAutoOpened.value = true;
  }
  await refreshUnifiedApproval(payload.request_id);
}


type WelcomeDonePayload = {
  skipped: boolean;
  deepseekApiKey: string;
  bochaApiKey: string;
  navigate: 'chat' | 'providers' | 'network' | 'console';
};

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

function buildSavedModelId(provider: string, model: string): string {
  return `${provider.trim()}:${model.trim()}`;
}

function buildSavedModelDisplayName(provider: string, model: string): string {
  return `${provider.trim()} - ${model.trim()}`;
}

function syncCurrentConfigToSavedModels(currentConfig: typeof config.value) {
  const provider = currentConfig.provider?.trim();
  const model = currentConfig.model?.trim();
  if (!provider || !model) {
    return;
  }

  const id = buildSavedModelId(provider, model);
  const existingIndex = savedModels.value.findIndex(
    (entry) => entry.provider === provider && entry.model === model
  );
  const nextEntry: SavedModel = {
    id,
    provider,
    model,
    apiBase: currentConfig.apiBase || "",
    apiKey: currentConfig.apiKey || "",
    displayName: buildSavedModelDisplayName(provider, model),
  };

  if (existingIndex === -1) {
    savedModels.value = [...savedModels.value, nextEntry];
    return;
  }

  const existing = savedModels.value[existingIndex];
  const merged: SavedModel = {
    ...existing,
    id,
    provider,
    model,
    apiBase: existing.apiBase || nextEntry.apiBase,
    apiKey: existing.apiKey || nextEntry.apiKey,
    displayName: existing.displayName || nextEntry.displayName,
  };

  if (JSON.stringify(existing) !== JSON.stringify(merged)) {
    const nextSavedModels = [...savedModels.value];
    nextSavedModels.splice(existingIndex, 1, merged);
    savedModels.value = nextSavedModels;
  }
}

function extractProviderConfigsFromRaw(parsed: RawConfigShape): Record<string, ProviderConfigEntry> {
  const configMap: Record<string, ProviderConfigEntry> = {};

  const builtins = (parsed.providers || {}) as Record<string, RawProviderConfig>;
  for (const [name, raw] of Object.entries(builtins)) {
    if (name === 'custom_providers' || !raw || typeof raw !== 'object') {
      continue;
    }
    configMap[name] = {
      apiKey: raw.api_key || '',
      apiBase: raw.api_base || '',
      source: 'providers',
    };
  }

  const customProviders = parsed.providers?.custom_providers || {};
  for (const [name, raw] of Object.entries(customProviders)) {
    if (!raw || typeof raw !== 'object') {
      continue;
    }
    configMap[name] = {
      apiKey: raw.api_key || '',
      apiBase: raw.api_base || '',
      source: 'custom_providers',
    };
  }

  return configMap;
}

function extractProviderConfigFromRaw(
  parsed: RawConfigShape,
  provider?: string | null
): RawProviderConfig | undefined {
  if (!provider) {
    return undefined;
  }

  return parsed.providers?.[provider] || parsed.providers?.custom_providers?.[provider];
}

function patchProviderConfigInRaw(parsed: RawConfigShape, nextConfig: typeof config.value) {
  const provider = nextConfig.provider?.trim();
  if (!provider) {
    return;
  }

  if (!parsed.agents) {
    parsed.agents = {};
  }
  if (!parsed.agents.defaults) {
    parsed.agents.defaults = {};
  }
  parsed.agents.defaults.provider = provider;
  parsed.agents.defaults.model = nextConfig.model?.trim() || null;

  if (!parsed.providers) {
    parsed.providers = {};
  }

  const currentMap = extractProviderConfigsFromRaw(parsed);
  const source =
    currentMap[provider]?.source ||
    (parsed.providers.custom_providers?.[provider] ? 'custom_providers' : 'providers');
  const container =
    source === 'custom_providers'
      ? (parsed.providers.custom_providers ||= {})
      : parsed.providers;

  const existing = container[provider] || {};
  container[provider] = {
    ...existing,
    api_key: nextConfig.apiKey?.trim() || undefined,
    api_base: nextConfig.apiBase?.trim() || null,
  };
}

function withTimeout<T>(task: Promise<T>, timeoutMs: number, label: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = window.setTimeout(() => {
      reject(new Error(`${label} timed out after ${timeoutMs}ms`));
    }, timeoutMs);

    task.then(
      (value) => {
        window.clearTimeout(timer);
        resolve(value);
      },
      (error) => {
        window.clearTimeout(timer);
        reject(error);
      }
    );
  });
}

function extractChatId(sessionKey: string): string {
  if (!sessionKey) {
    return '';
  }
  const idx = sessionKey.indexOf(':');
  if (idx === -1 || idx === sessionKey.length - 1) {
    return sessionKey;
  }
  return sessionKey.slice(idx + 1);
}

function parseSessionTimestamp(updatedAt?: string | null, createdAt?: string | null): number {
  const parsed = Date.parse(updatedAt || createdAt || '');
  return Number.isFinite(parsed) ? parsed : 0;
}

function parseMessageTimestamp(rawTimestamp?: string): number {
  const parsed = Date.parse(rawTimestamp || '');
  return Number.isFinite(parsed) ? parsed : Date.now();
}

function compactPreview(content: string, maxChars = 120): string {
  const normalized = content.trim().replace(/\s+/g, ' ');
  if (!normalized) return '';
  const preview = normalized.slice(0, maxChars);
  return normalized.length > maxChars ? `${preview}...` : preview;
}

function fallbackSessionTitle(content?: string): string {
  const normalized = compactPreview(content || '', 20);
  return normalized || '新会话';
}

function sortSessionsByTimestamp() {
  sessions.value = [...sessions.value].sort((a, b) => b.timestamp - a.timestamp);
}

function upsertSession(session: SessionInfo) {
  const index = sessions.value.findIndex((item) => item.session_key === session.session_key);
  if (index === -1) {
    sessions.value = [session, ...sessions.value];
  } else {
    const next = [...sessions.value];
    next[index] = {
      ...next[index],
      ...session,
    };
    sessions.value = next;
  }
  sortSessionsByTimestamp();
}

function currentVisibleMessages(): Message[] {
  return messages.value.filter((message) => {
    if (!['user', 'agent', 'tool'].includes(message.role)) return false;
    return Boolean(message.content?.trim());
  });
}

function syncCurrentSessionListEntry(seedContent?: string) {
  const visibleMessages = currentVisibleMessages();
  const lastVisible = [...visibleMessages].reverse().find((message) => message.content?.trim());
  const existing = sessions.value.find((session) => session.session_key === currentSessionKey.value);
  const fallbackTitle = existing?.title || fallbackSessionTitle(seedContent || visibleMessages[0]?.content);
  upsertSession({
    session_key: currentSessionKey.value,
    chat_id: currentChatId.value,
    snippet: compactPreview(lastVisible?.content || seedContent || currentChatId.value, 120),
    last_message: compactPreview(lastVisible?.content || seedContent || '', 120) || undefined,
    timestamp: lastVisible?.timestamp || Date.now(),
    title: existing?.title || fallbackTitle,
    message_count: visibleMessages.length,
    title_generated: existing?.title_generated || false,
    title_manually_set: existing?.title_manually_set || false,
    pinned: existing?.pinned || false,
  });
}

async function maybeGenerateCurrentSessionTitle() {
  const sessionKey = currentSessionKey.value;
  const session = sessions.value.find((item) => item.session_key === sessionKey);
  if (!session || session.title_manually_set || session.title_generated) {
    return;
  }
  if (titleGenerationInFlight.value.has(sessionKey)) {
    return;
  }
  const firstUser = messages.value.find((message) => message.role === 'user' && message.content.trim());
  const firstAssistant = messages.value.find((message) => message.role === 'agent' && message.content.trim());
  if (!firstUser || !firstAssistant) {
    return;
  }

  titleGenerationInFlight.value.add(sessionKey);
  try {
    const response = await invoke<{
      title?: string;
      title_generated?: boolean;
      title_manually_set?: boolean;
    }>('generate_session_title', {
      sessionKey,
      payload: {
        firstUserMessage: firstUser.content,
        firstAssistantMessage: firstAssistant.content,
      },
    });
    upsertSession({
      ...session,
      title: typeof response?.title === 'string' && response.title.trim()
        ? response.title.trim()
        : session.title || fallbackSessionTitle(firstUser.content),
      title_generated: response?.title_generated === true,
      title_manually_set: response?.title_manually_set === true,
      timestamp: Date.now(),
    });
  } catch (error) {
    console.warn('Failed to generate session title:', error);
    upsertSession({
      ...session,
      title: session.title || fallbackSessionTitle(firstUser.content),
      title_generated: false,
      title_manually_set: false,
    });
  } finally {
    titleGenerationInFlight.value.delete(sessionKey);
  }
}

function mapSessionRole(
  role: string
): Message['role'] | null {
  if (role === 'assistant') return 'agent';
  if (role === 'user' || role === 'system' || role === 'tool') return role;
  return null;
}

function hasValue(value: unknown): boolean {
  if (value === null || value === undefined) return false;
  if (Array.isArray(value)) return value.length > 0;
  if (typeof value === 'object') return Object.keys(value as Record<string, unknown>).length > 0;
  return true;
}

function isChecklistCardContent(content: string): boolean {
  if (!content) return false;
  try {
    const parsed = JSON.parse(content) as { kind?: string; plan_items?: unknown };
    return parsed.kind === 'checklist' && Array.isArray(parsed.plan_items);
  } catch {
    return false;
  }
}

function extractToolName(
  role: string,
  name?: string | null,
  toolCalls?: serdeJsonValue[] | null,
  fallbackToolName?: string
): string | undefined {
  if (name && name.trim()) return name.trim();
  if (!toolCalls || toolCalls.length === 0) {
    if (fallbackToolName && fallbackToolName.trim()) return fallbackToolName.trim();
    return role === 'tool' ? t('app.unknownTool') : undefined;
  }
  const firstCall = toolCalls[0];
  const maybeFn = firstCall?.function as Record<string, unknown> | undefined;
  const fnName = maybeFn?.name;
  if (typeof fnName === 'string' && fnName.trim()) return fnName.trim();
  if (fallbackToolName && fallbackToolName.trim()) return fallbackToolName.trim();
  return role === 'tool' ? t('app.unknownTool') : undefined;
}

function buildHistoricalToolNameIndex(messages: BackendChatMessage[]): Map<string, string> {
  const names = new Map<string, string>();
  for (const message of messages) {
    if (message.role !== 'assistant' || !message.tool_calls) continue;
    for (const call of message.tool_calls) {
      const callId = typeof call.id === 'string'
        ? call.id
        : typeof call.tool_call_id === 'string'
          ? call.tool_call_id
          : undefined;
      const maybeFunction = call.function as Record<string, unknown> | undefined;
      const callName = typeof maybeFunction?.name === 'string'
        ? maybeFunction.name
        : typeof call.name === 'string'
          ? call.name
          : undefined;
      if (callId && callName?.trim()) {
        names.set(callId, callName.trim());
      }
    }
  }
  return names;
}

function extractToolArgs(toolCalls?: serdeJsonValue[] | null): string | undefined {
  if (!toolCalls || toolCalls.length === 0) return undefined;
  const firstCall = toolCalls[0];
  const maybeFn = firstCall?.function as Record<string, unknown> | undefined;
  const fnArgs = maybeFn?.arguments ?? firstCall?.args;
  if (!hasValue(fnArgs)) return undefined;
  if (typeof fnArgs === 'string') return fnArgs;
  try {
    return JSON.stringify(fnArgs, null, 2);
  } catch {
    return String(fnArgs);
  }
}

function buildRawMeta(msg: BackendChatMessage): Record<string, unknown> | undefined {
  const rawMeta: Record<string, unknown> = {};
  if (hasValue(msg.tool_call_id)) rawMeta.tool_call_id = msg.tool_call_id;
  if (hasValue(msg.tool_calls)) rawMeta.tool_calls = msg.tool_calls;
  if (hasValue(msg.name)) rawMeta.name = msg.name;
  if (hasValue(msg.thinking_blocks)) rawMeta.thinking_blocks = msg.thinking_blocks;
  if (hasValue(msg.metadata)) rawMeta.metadata = msg.metadata;
  if (Object.keys(rawMeta).length === 0) return undefined;
  return rawMeta;
}

function mapBackendMessageToUi(msg: BackendChatMessage, fallbackToolName?: string): Message | null {
  const mappedRole = mapSessionRole(msg.role);
  if (!mappedRole) {
    return null;
  }

  const toolName = extractToolName(msg.role, msg.name, msg.tool_calls, fallbackToolName);
  const toolArgs = extractToolArgs(msg.tool_calls);
  const rawMeta = buildRawMeta(msg);
  const toolResult = mappedRole === 'tool' ? (msg.content || '') : undefined;
  const toolStatus = mappedRole === 'tool'
    ? (/^error\b/i.test(msg.content || '') ? 'error' : 'success')
    : undefined;

  return {
    id: generateMessageId(),
    role: mappedRole,
    content: msg.content || '',
    reasoning: msg.reasoning_content || undefined,
    timestamp: parseMessageTimestamp(msg.timestamp),
    emotion: mappedRole === 'agent' ? 'normal' : undefined,
    toolName,
    toolArgs,
    toolResult,
    toolStatus,
    toolCallId: msg.tool_call_id || undefined,
    rawMeta,
    fromHistory: true,
  };
}

function getSessionCacheKeys(chatId: string): string[] {
  if (!chatId) return [];
  const normalized = chatId.includes(':') ? chatId : `gui:${chatId}`;
  const keys = [normalized];
  if (chatId !== normalized) {
    keys.push(chatId);
  }
  return keys.map((key) => `${SESSION_CACHE_PREFIX}${key}`);
}

function readSessionFromCache(chatId: string): BackendSessionHistory | null {
  try {
    for (const key of getSessionCacheKeys(chatId)) {
      const raw = localStorage.getItem(key);
      if (!raw) continue;
      const parsed = JSON.parse(raw) as SessionCacheEntry;
      if (!parsed || !parsed.session || !Array.isArray(parsed.session.messages)) continue;
      if (!Number.isFinite(parsed.cachedAt) || Date.now() - parsed.cachedAt > SESSION_CACHE_TTL_MS) {
        localStorage.removeItem(key);
        continue;
      }
      return parsed.session;
    }
  } catch (e) {
    console.warn('Failed to read session cache:', e);
  }
  return null;
}

function writeSessionToCache(session: BackendSessionHistory) {
  try {
    if (!session?.key) return;
    const entry: SessionCacheEntry = {
      session,
      cachedAt: Date.now(),
    };
    localStorage.setItem(
      `${SESSION_CACHE_PREFIX}${session.key}`,
      JSON.stringify(entry)
    );
  } catch (e) {
    console.warn('Failed to write session cache:', e);
  }
}

function generateMessageId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return `msg-${crypto.randomUUID()}`;
  }
  return `msg-${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

function generateChatId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return `chat-${crypto.randomUUID()}`;
  }
  return `chat-${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

function generateStreamRequestId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  return `stream-${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

function closeStreamingPlaceholder(removeIfEmpty = false) {
  for (let i = messages.value.length - 1; i >= 0; i--) {
    const lastMsg = messages.value[i];
    if (lastMsg.role !== 'agent' || !lastMsg.isStreaming) {
      continue;
    }
    if (removeIfEmpty && !lastMsg.content && !lastMsg.reasoning) {
      messages.value.splice(i, 1);
      return;
    }
    lastMsg.isStreaming = false;
    lastMsg.isThinking = false;
    return;
  }
}

function removeStreamingAssistantPlaceholder() {
  for (let i = messages.value.length - 1; i >= 0; i--) {
    const message = messages.value[i];
    if (message.role !== 'agent' || !message.isStreaming) {
      continue;
    }
    if (!message.content) {
      messages.value.splice(i, 1);
    } else {
      message.isStreaming = false;
      message.isThinking = false;
    }
    return;
  }
}

// Load saved models from localStorage
onMounted(() => {
  const storedModels = localStorage.getItem(SAVED_MODELS_KEY);
  if (storedModels) {
    try {
      savedModels.value = JSON.parse(storedModels);
    } catch (e) {
      console.error("Failed to parse saved models", e);
    }
  }
  syncCurrentConfigToSavedModels(config.value);

  const storedChatPrefs = localStorage.getItem(HISTORY_PREFS_KEY);
  if (storedChatPrefs) {
    try {
      chatDisplayPrefs.value = {
        ...defaultChatDisplayPrefs,
        ...JSON.parse(storedChatPrefs),
      };
    } catch (e) {
      console.error('Failed to parse chat display preferences', e);
    }
  }
});

// Save models to localStorage whenever they change
watch(savedModels, (newVal) => {
  localStorage.setItem(SAVED_MODELS_KEY, JSON.stringify(newVal));
}, { deep: true });

watch(() => ({ ...config.value }), (newConfig) => {
  syncCurrentConfigToSavedModels(newConfig);
}, { deep: true });

watch(chatDisplayPrefs, (newVal) => {
  localStorage.setItem(HISTORY_PREFS_KEY, JSON.stringify(newVal));
}, { deep: true });

function updateSavedModels(models: SavedModel[]) {
  savedModels.value = models;
}

function updateChatDisplayPrefs(prefs: ChatDisplayPrefs) {
  chatDisplayPrefs.value = {
    ...defaultChatDisplayPrefs,
    ...prefs,
  };
}

function isAwaitingApprovalPhase(plan: PlanRuntimeState): boolean {
  return plan.phase === 'AwaitingApproval' || plan.status === 'AwaitingApproval';
}

function isExecutingPhase(plan: PlanRuntimeState): boolean {
  return plan.phase === 'Execute'
    || plan.phase === 'Verify'
    || plan.status === 'Approved'
    || plan.status === 'Executing'
    || plan.status === 'Verifying';
}

function isTerminalPhase(plan: PlanRuntimeState): boolean {
  return plan.phase === 'Completed'
    || plan.phase === 'Failed'
    || plan.phase === 'Partial'
    || plan.status === 'Closed'
    || plan.status === 'Completed'
    || plan.status === 'Failed'
    || plan.status === 'Partial';
}

function syncPlanRuntime(plan: PlanRuntimeState | null) {
  if (!plan) {
    activePlanRuntime.value = null;
    pendingApprovalPlan.value = null;
    executingPlan.value = null;
    return;
  }
  // Normalize report statuses into the phases the chat card/bar understand.
  // Keep markdown/summary/strategy byte-stable for revision_hash on approve;
  // only display fields (title/goal) may be sanitized.
  const normalized: PlanRuntimeState = {
    ...plan,
    title: sanitizePlanText(plan.title) || plan.title || 'Plan',
    goal: sanitizePlanText(plan.goal) || plan.goal,
  };
  if (isAwaitingApprovalPhase(normalized)) {
    normalized.phase = 'AwaitingApproval';
    normalized.status = 'AwaitingApproval';
    activePlanRuntime.value = normalized;
    pendingApprovalPlan.value = normalized;
    executingPlan.value = null;
    return;
  }
  if (isExecutingPhase(normalized)) {
    if (normalized.phase !== 'Verify') normalized.phase = 'Execute';
    activePlanRuntime.value = normalized;
    executingPlan.value = normalized;
    pendingApprovalPlan.value = null;
    return;
  }
  if (isTerminalPhase(normalized)) {
    // Terminal plans must not keep occupying the chat's active-plan bar.
    activePlanRuntime.value = null;
    pendingApprovalPlan.value = null;
    executingPlan.value = null;
    return;
  }
  // Unknown phase: keep visible as active, but if it has markdown treat as pending review.
  if ((normalized.markdown || normalized.summary || normalized.strategy) && normalized.revision != null) {
    normalized.phase = 'AwaitingApproval';
    normalized.status = 'AwaitingApproval';
    activePlanRuntime.value = normalized;
    pendingApprovalPlan.value = normalized;
    executingPlan.value = null;
    return;
  }
  activePlanRuntime.value = normalized;
}

function sanitizePlanText(value: string | null | undefined): string {
  if (!value) return '';
  // Drop UTF-8 replacement chars (common "计��报告" corruption) and trim.
  // Display-only: do not use this on plan body markdown used for revision_hash.
  return value.replace(/\uFFFD/g, '').trim();
}

/** Keep plan body stable for server-side revision_hash (trailing newline matters). */
function preservePlanMarkdown(value: string | null | undefined): string {
  if (typeof value !== 'string') return '';
  return value;
}

function planReportId(value: unknown): string | null {
  if (typeof value === 'string' && value.length > 0) return value;
  if (value && typeof value === 'object') {
    const obj = value as Record<string, unknown>;
    if (typeof obj['0'] === 'string' && obj['0'].length > 0) return obj['0'];
    if (typeof obj.id === 'string' && obj.id.length > 0) return obj.id;
  }
  return null;
}

function planReportSessionKey(value: unknown): string | null {
  if (!value || typeof value !== 'object') return null;
  const root = value as Record<string, unknown>;
  const report = root.report && typeof root.report === 'object'
    ? root.report as Record<string, unknown>
    : root;
  return typeof report.session_key === 'string' && report.session_key.trim()
    ? report.session_key.trim()
    : null;
}

function planRuntimeFromReportPayload(payload: unknown): PlanRuntimeState | null {
  if (!payload || typeof payload !== 'object') return null;
  const root = payload as Record<string, unknown>;

  // Accept:
  // 1) SSE envelope `{ report: PlanReportDetail }`
  // 2) bare `PlanReportDetail` `{ report, revision }`
  // 3) double-wrapped `{ report: { report: PlanReportDetail } }`
  let detail: Record<string, unknown> = root;
  if (root.report && typeof root.report === 'object') {
    const level1 = root.report as Record<string, unknown>;
    if (level1.report && typeof level1.report === 'object' && level1.revision) {
      detail = level1;
    } else if (level1.revision || level1.markdown) {
      detail = level1.revision ? level1 : root;
    } else if (root.revision) {
      detail = root;
    } else {
      detail = level1;
    }
  }

  const reportMeta = (detail.report && typeof detail.report === 'object'
    ? detail.report
    : detail) as {
    id?: unknown;
    current_revision?: number;
    status?: string;
    created_at?: string;
    updated_at?: string;
  };
  const revisionMeta = (detail.revision && typeof detail.revision === 'object'
    ? detail.revision
    : detail) as {
    title?: string;
    markdown?: string;
    revision?: number;
  };

  const id = planReportId(reportMeta.id);
  // Exact body from the report event — must match server-stored bytes for revision_hash.
  const markdown = preservePlanMarkdown(
    revisionMeta.markdown || (typeof detail.markdown === 'string' ? detail.markdown : ''),
  );
  if (!id || !markdown.trim()) return null;

  const titleFromMarkdown = markdown
    .split('\n')
    .map((line) => line.trim())
    .find((line) => line.startsWith('# '))
    ?.slice(2)
    .trim();
  let title = sanitizePlanText(revisionMeta.title || titleFromMarkdown || '') || 'Plan';
  // Recover from truncated mojibake titles like "计报告".
  if (title.includes('报告') && title.length < 4) {
    title = titleFromMarkdown && titleFromMarkdown.length >= 4 ? titleFromMarkdown : '计划报告';
  }
  const goalLine = markdown
    .split('\n')
    .map((line) => line.trim())
    .find((line) => line.length > 0 && !line.startsWith('#'));
  const validation_issues = planReportValidationIssues(markdown);
  // Report-ready always means pending user approval in the replacement model.
  return {
    plan_id: id,
    revision: revisionMeta.revision ?? reportMeta.current_revision ?? null,
    title,
    goal: goalLine || title || 'Markdown plan report',
    phase: 'AwaitingApproval',
    status: 'AwaitingApproval',
    strategy: markdown,
    summary: markdown,
    markdown,
    validation_issues: validation_issues.length ? validation_issues : undefined,
    steps: [],
    todos: [],
    created_at: reportMeta.created_at || '',
    updated_at: reportMeta.updated_at || '',
  };
}

function sessionRoute(sessionKey: string): { channel: string; chatId: string } {
  const separator = sessionKey.indexOf(':');
  if (separator > 0 && separator < sessionKey.length - 1) {
    return {
      channel: sessionKey.slice(0, separator),
      chatId: sessionKey.slice(separator + 1),
    };
  }
  return { channel: currentChannel.value, chatId: extractChatId(sessionKey) };
}

type PlanApprovalTarget = {
  plan: PlanRuntimeState;
  sessionKey: string;
};

async function runPlanApproval(
  payload: {
    contextPolicy: 'retain' | 'compact' | 'clear';
    todoPolicy: 'Never' | 'Optional' | 'Always';
    materializeTodos: boolean;
  },
  target?: PlanApprovalTarget,
): Promise<PlanRuntimeState> {
  if (approvingPlan.value) throw new Error('Another plan approval is already processing.');
  if (isTyping.value) throw new Error('Another response is already streaming.');
  approvingPlan.value = true;
  try {
    const pending = target?.plan ?? pendingApprovalPlan.value;
    const sessionKey = target?.sessionKey
      || pendingApprovalSessionKey.value
      || currentSessionKey.value;
    if (!pending) throw new Error('No pending plan approval is available.');
    if (pending.revision == null) {
      throw new Error('Plan revision is unavailable; refresh before approving.');
    }
    if (target) {
      pendingApprovalSessionKey.value = sessionKey;
      syncPlanRuntime(pending);
    }
    const result = await approveActivePlanExecution({
      session_key: sessionKey,
      plan_id: pending.plan_id,
      expected_revision: pending.revision,
      markdown: pending.markdown || pending.summary || '',
      todo_policy: payload.todoPolicy,
      materialize_todos: payload.materializeTodos,
      context_policy: payload.contextPolicy,
    });
    const approved = {
      ...result.plan,
      plan_id: result.plan.plan_id || pending.plan_id,
      markdown: result.plan.markdown || pending.markdown,
      summary: result.plan.summary || pending.summary || pending.markdown || '',
      strategy: result.plan.strategy ?? pending.strategy ?? pending.markdown ?? null,
      steps: result.plan.steps ?? [],
      todos: result.plan.todos ?? [],
    };
    if (!isExecutingPhase(approved)) {
      throw new Error('Approval did not return an executing backend state; refresh and retry.');
    }

    // The backend approval is durable before the stream starts. Reflect that
    // authority immediately so a stream-start failure cannot resurrect a
    // stale pending card.
    syncPlanRuntime({ ...approved, initialization_status: 'Ready' });
    planContinuationContext.value = {
      planId: approved.plan_id,
      revision: approved.revision ?? pending.revision,
      executionId: approved.execution_id ?? null,
      sessionKey,
    };
    planContinuationError.value = null;
    try {
      await continueApprovedPlanExecution(
        approved.plan_id,
        approved.revision ?? pending.revision,
        approved.execution_id ?? undefined,
        sessionKey,
      );
    } catch (error) {
      planContinuationError.value = error instanceof Error ? error.message : String(error);
      throw error;
    }
    return { ...approved, initialization_status: 'Ready' };
  } finally {
    approvingPlan.value = false;
  }
}

async function approvePlanExecution(payload: {
  contextPolicy: 'retain' | 'compact' | 'clear';
  todoPolicy: 'Never' | 'Optional' | 'Always';
  materializeTodos: boolean;
}) {
  if (approvingPlan.value) return;
  try {
    await runPlanApproval(payload);
  } catch (error) {
    messages.value.push({
      id: generateMessageId(),
      role: 'system',
      content: `${t('app.errorPrefix')}${error}`,
      timestamp: Date.now(),
    });
  }
}

async function continueApprovedPlanExecution(
  planId?: string,
  revision?: number,
  executionId?: string,
  targetSessionKey = currentSessionKey.value,
) {
  if (isTyping.value) {
    throw new Error('Another response is already streaming.');
  }
  if (!isTauri()) {
    return;
  }

  isTyping.value = true;
  suppressNextStopError.value = false;
  closeStreamingPlaceholder(true);
  const streamRequestId = generateStreamRequestId();
  activeStreamRequestId.value = streamRequestId;
  messages.value.push({
    id: generateMessageId(),
    role: 'agent',
    content: '',
    isStreaming: true,
    timestamp: Date.now(),
    emotion: currentEmotion.value,
  });

  try {
    const route = sessionRoute(targetSessionKey);
    await invoke('continue_approved_plan_execution', {
      channel: route.channel,
      chatId: route.chatId,
      streamRequestId,
      planId,
      revision,
      executionId,
    });
  } catch (error) {
    activeStreamRequestId.value = null;
    removeStreamingAssistantPlaceholder();
    isTyping.value = false;
    throw error;
  }
}

async function resumePlanExecution() {
  const context = planContinuationContext.value;
  if (!context || !planContinuationError.value || approvingPlan.value || isTyping.value) return;
  approvingPlan.value = true;
  planContinuationError.value = null;
  try {
    await continueApprovedPlanExecution(
      context.planId,
      context.revision,
      context.executionId ?? undefined,
      context.sessionKey,
    );
  } catch (error) {
    planContinuationError.value = error instanceof Error ? error.message : String(error);
    messages.value.push({
      id: generateMessageId(),
      role: 'system',
      content: `${t('app.errorPrefix')}${error}`,
      timestamp: Date.now(),
    });
  } finally {
    approvingPlan.value = false;
  }
}

async function restoreActivePlanRuntime() {
  if (!isTauri()) return;
  try {
    const plan = await invoke<PlanDetail | null>('get_active_plan', {
      sessionKey: currentSessionKey.value || `gui:${currentChatId.value}`,
    });
    if (!plan || !plan.id) {
      // Do not wipe a live pending approval card when the backend returns empty
      // (race after plan-report-ready, or transient list failure).
      if (pendingApprovalPlan.value && isAwaitingApprovalPhase(pendingApprovalPlan.value)) {
        return;
      }
      syncPlanRuntime(null);
      return;
    }
    syncPlanRuntime({
      plan_id: plan.id,
      revision: plan.revision,
      title: sanitizePlanText(plan.title) || plan.title,
      goal: sanitizePlanText(plan.goal) || plan.goal,
      phase: plan.phase,
      status: plan.status,
      strategy: plan.strategy,
      summary: plan.summary || plan.markdown || `${plan.title}: ${plan.goal}`,
      markdown: plan.markdown || plan.summary || plan.strategy || plan.goal,
      steps: (plan.steps ?? []).map((step) => ({
        id: step.id,
        ordinal: step.ordinal,
        title: step.title,
        rationale: step.rationale,
        expected_output: step.expected_output,
        status: step.status,
      })),
      todos: (plan.todos ?? []).map((todo) => ({
        id: todo.id,
        plan_step_id: todo.plan_step_id,
        title: todo.title,
        detail: todo.detail,
        status: todo.status,
        priority: todo.priority,
        evidence_ref: todo.evidence_ref,
        block_reason: todo.block_reason,
        updated_at: todo.updated_at,
      })),
      created_at: plan.created_at,
      updated_at: plan.updated_at,
    });
  } catch (error) {
    console.warn('Failed to restore active plan runtime:', error);
  }
}

async function revokePlanExecution(feedback = '') {
  const plan = pendingApprovalPlan.value;
  if (!plan) return;
  if (feedback.trim() && isTyping.value) {
    messages.value.push({
      id: generateMessageId(),
      role: 'system',
      content: '请等待当前回复完成后再提交计划修改意见。',
      timestamp: Date.now(),
    });
    return;
  }
  try {
    const reopened = await returnActivePlanToDraft(
      currentSessionKey.value || `gui:${currentChatId.value}`,
    );
    syncPlanRuntime(reopened);
    if (feedback.trim()) await sendMessage(feedback.trim(), undefined, 'plan');
  } catch (error) {
    messages.value.push({
      id: generateMessageId(),
      role: 'system',
      content: `${t('app.errorPrefix')}${error}`,
      timestamp: Date.now(),
    });
  }
}

async function sendMessage(content: string, attachments?: FileAttachmentDto[], mode: ExecMode = 'agent', permissionMode?: 'cautious' | 'smart' | 'trusted') {
  if (workspaceSwitching.value) {
    showAppToast('工作区切换进行中，请稍候。', 'error', 4000);
    return;
  }
  if (!content.trim() && (!attachments || attachments.length === 0)) return;
  if (isTyping.value) return;
  if (content.trim() === '/stop') {
    await stopMessage();
    return;
  }

  console.log('[App] Sending message with current config:', {
      ...config.value,
      apiKey: config.value.apiKey ? `${config.value.apiKey.substring(0, 8)}...` : 'undefined'
  });

  const attachmentFileIds = attachments?.map(a => a.file_id);
  const userMsg: Message = {
    id: generateMessageId(),
    role: 'user',
    content: content,
    timestamp: Date.now(),
    attachments: attachmentFileIds,
  };
  messages.value.push(userMsg);
  syncCurrentSessionListEntry(content);
  
  isTyping.value = true;
  suppressNextStopError.value = false;
  closeStreamingPlaceholder(true);
  const streamRequestId = generateStreamRequestId();
  activeStreamRequestId.value = streamRequestId;
  
  // Create a placeholder for the agent response
  messages.value.push({ 
    id: generateMessageId(),
    role: 'agent', 
    content: '', 
    isStreaming: true, 
    timestamp: Date.now(),
    emotion: currentEmotion.value
  });

  try {
    if (!isTauri()) {
        console.warn('Running in browser, mocking sendMessage');
        setTimeout(() => {
            const lastMsg = messages.value[messages.value.length - 1];
            if (lastMsg && lastMsg.role === 'agent') {
                lastMsg.content = t('app.mockResponse', { content });
                lastMsg.isStreaming = false;
                isTyping.value = false;
                activeStreamRequestId.value = null;
                syncCurrentSessionListEntry(content);
            }
        }, 1000);
        return;
    }

    const approvalPolicy =
      permissionMode === 'cautious'
        ? 'on-request'
        : permissionMode === 'trusted'
          ? 'unless-trusted'
          : permissionMode === 'smart'
            ? 'on-failure'
            : undefined;
    await invoke("send_message", {
      message: content,
      channel: currentChannel.value,
      chatId: currentChatId.value,
      attachments: attachmentFileIds,
      mode,
      streamRequestId,
      approvalPolicy,
    });
  } catch (error) {
    console.error("Failed to send message:", error);
    activeStreamRequestId.value = null;
    
    // Remove the placeholder agent message
    if (messages.value.length > 0) {
        const lastMsg = messages.value[messages.value.length - 1];
        if (lastMsg.role === 'agent' && lastMsg.isStreaming) {
            messages.value.pop();
        }
    }

    messages.value.push({ 
      id: generateMessageId(),
      role: 'system', 
      content: `${t('app.errorPrefix')}${error}`, 
      timestamp: Date.now() 
    });
    
    isTyping.value = false;
  }
}

async function regenerateMessage(messageId: string) {
  if (isTyping.value) return;

  // 1. 定位要覆盖的 agent 消息
  const targetIndex = messages.value.findIndex(
    (m) => m.id === messageId && m.role === 'agent'
  );
  if (targetIndex === -1) return;

  // 2. 找到它前面最近一条 user 消息（作为重试 prompt）
  let userIndex = -1;
  for (let i = targetIndex - 1; i >= 0; i--) {
    if (messages.value[i].role === 'user') {
      userIndex = i;
      break;
    }
  }
  if (userIndex === -1) return;

  const userMsg = messages.value[userIndex];

  // 3. 生成新的流式请求 ID
  const streamRequestId = generateStreamRequestId();
  activeStreamRequestId.value = streamRequestId;
  isTyping.value = true;
  suppressNextStopError.value = false;

  // 4. 就地清空目标 agent 消息，标记为 streaming（覆盖而非新增）
  const target = messages.value[targetIndex];
  target.content = '';
  target.reasoning = '';
  target.isThinking = false;
  target.isStreaming = true;
  target.timestamp = Date.now();

  // 5. 截断目标 agent 之后的所有消息，确保现有流式监听器写入正确位置
  messages.value.splice(targetIndex + 1);

  try {
    if (!isTauri()) {
      console.warn('Running in browser, mocking regenerateMessage');
      setTimeout(() => {
        if (activeStreamRequestId.value !== streamRequestId) return;
        target.content = t('app.mockResponse', { content: userMsg.content });
        target.isStreaming = false;
        target.isThinking = false;
        isTyping.value = false;
        activeStreamRequestId.value = null;
      }, 1000);
      return;
    }

    await invoke("send_message", {
      message: userMsg.content,
      channel: currentChannel.value,
      chatId: currentChatId.value,
      attachments: userMsg.attachments,
      mode: 'agent',
      streamRequestId,
    });
  } catch (error) {
    console.error("Failed to regenerate message:", error);
    activeStreamRequestId.value = null;
    target.isStreaming = false;
    target.isThinking = false;
    messages.value.push({
      id: generateMessageId(),
      role: 'system',
      content: `${t('app.errorPrefix')}${error}`,
      timestamp: Date.now()
    });
    isTyping.value = false;
  }
}

async function stopMessage() {
  if (!isTyping.value) return;

  try {
    if (!isTauri()) {
      const hadVisibleResponse = messages.value.some(
        (message) => message.role === 'agent' && message.isStreaming && !!message.content,
      );
      removeStreamingAssistantPlaceholder();
      isTyping.value = false;
      if (!hadVisibleResponse) {
        messages.value.push({
          id: generateMessageId(),
          role: 'agent',
          content: t('app.stoppedMessage'),
          timestamp: Date.now(),
          emotion: currentEmotion.value,
        });
      }
      return;
    }

    await invoke("stop_generation", {
      channel: currentChannel.value,
      chatId: currentChatId.value,
    });
    suppressNextStopError.value = true;
    const hadVisibleResponse = messages.value.some(
      (message) => message.role === 'agent' && message.isStreaming && !!message.content,
    );
    removeStreamingAssistantPlaceholder();
    isTyping.value = false;
    activeStreamRequestId.value = null;
    if (!hadVisibleResponse) {
      messages.value.push({
        id: generateMessageId(),
        role: 'agent',
        content: t('app.stoppedMessage'),
        timestamp: Date.now(),
        emotion: currentEmotion.value,
      });
    }
    syncCurrentSessionListEntry();
  } catch (error) {
    messages.value.push({
      id: generateMessageId(),
      role: 'system',
      content: t('app.stopFailed', { error }),
      timestamp: Date.now()
    });
  }
}

const clearMessages = () => {
  currentChatId.value = generateChatId();
  currentSessionKey.value = `gui:${currentChatId.value}`;
  activeStreamRequestId.value = null;
  isTyping.value = false;
  syncPlanRuntime(null);
  messages.value = [
    {
      id: generateMessageId(),
      role: 'agent',
      content: t('app.cleared'),
      timestamp: Date.now(),
      emotion: 'happy'
    }
  ];
  upsertSession({
    session_key: currentSessionKey.value,
    chat_id: currentChatId.value,
    snippet: '新会话',
    last_message: undefined,
    timestamp: Date.now(),
    title: '新会话',
    message_count: 0,
    title_generated: false,
    title_manually_set: false,
    pinned: false,
  });
};

async function refreshSessions(): Promise<boolean> {
  if (!isTauri()) return false;
  try {
    const fetched = await withTimeout(
      invoke<BackendSessionInfo[]>("get_sessions"),
      STARTUP_TASK_TIMEOUT_MS,
      "get_sessions"
    );
    if (fetched && Array.isArray(fetched)) {
      const mapped = fetched.map((session) => {
        const chatId = extractChatId(session.key);
        return {
          session_key: session.key,
          chat_id: chatId,
          snippet: compactPreview(session.last_message || session.title || chatId || session.key || '...', 120),
          last_message: session.last_message || undefined,
          timestamp: parseSessionTimestamp(session.updated_at, session.created_at),
          title: session.title || undefined,
          message_count: session.message_count || 0,
          title_generated: session.title_generated === true,
          title_manually_set: session.title_manually_set === true,
          pinned: session.pinned === true,
          workspace_id: session.workspace_id || undefined,
          channel: session.channel || undefined,
          kind: session.kind || 'root',
          root_session_key: session.root_session_key || undefined,
          parent_session_key: session.parent_session_key || undefined,
          branch_label: session.branch_label || undefined,
          legacy: session.legacy === true,
        } satisfies SessionInfo;
      });
      sessions.value = mapped
        .filter((session) => !locallyDeletedSessionKeys.value.has(session.session_key))
        .sort((a, b) => b.timestamp - a.timestamp);
      return true;
    }
  } catch (e) {
    console.error("Failed to fetch sessions:", e);
  }
  return false;
}

async function switchWorkspaceAction(root: string): Promise<boolean> {
  const blocked = workspaceSwitchBlockedReason.value;
  if (blocked) {
    showAppToast(blocked, 'error', 5000);
    throw new Error(blocked);
  }
  if (!isTauri()) {
    const error = '工作区原子切换仅支持桌面 Tauri 运行时。';
    showAppToast(error, 'error', 5000);
    throw new Error(error);
  }

  workspaceSwitching.value = true;
  try {
    // Refresh the active workspace collection before stopping its runtime. The
    // session authority already persisted completed turns; this snapshot keeps
    // the GUI list aligned with the committed old workspace at the boundary.
    syncCurrentSessionListEntry();
    if (!(await refreshSessions())) {
      throw new Error('旧工作区会话快照读取失败，切换已取消。');
    }

    const request: WorkspaceSwitchRequest = {
      root,
      activeTurn: isTyping.value,
      planActive: Boolean(
        activePlanRuntime.value
        || pendingApprovalPlan.value
        || executingPlan.value
        || approvingPlan.value,
      ),
      approvalPending: approvalPendingCount.value > 0,
      askUserPending: pendingQuestions.value.length > 0,
    };
    const nextStatus = await switchWorkspaceApi(request);
    applyWorkspaceStatus(nextStatus);

    locallyDeletedSessionKeys.value = new Set();
    unifiedApprovals.value = [];
    approvalDetails.value = {};
    pendingQuestions.value = [];
    clearMessages();

    const sessionsRefreshed = await refreshSessions();
    if (sessionsRefreshed) {
      await restoreLatestGuiChatOnStartup();
    } else {
      showAppToast('工作区已切换，但新工作区历史刷新失败，请稍后重试。', 'error', 6000);
    }
    await restoreActivePlanRuntime();
    if (sessionsRefreshed) showAppToast('已切换工作区，并载入该工作区会话');
    return true;
  } catch (cause) {
    const message = cause instanceof Error ? cause.message : String(cause);
    showAppToast(message, 'error', 6000);
    throw cause instanceof Error ? cause : new Error(message);
  } finally {
    workspaceSwitching.value = false;
  }
}

/** GUI channel sessions only, already newest-first (same order as `sessions`). */
function listGuiSessionsForRestore(): SessionInfo[] {
  return sessions.value.filter((s) => s.session_key.startsWith('gui:'));
}

/**
 * After `refreshSessions`, load the most recently updated `gui:` session; try older GUI sessions if the first fails.
 */
async function restoreLatestGuiChatOnStartup() {
  const guiSessions = listGuiSessionsForRestore();
  if (guiSessions.length === 0) {
    return;
  }
  for (const { session_key } of guiSessions) {
    messages.value = [];
    const ok = await loadSession(session_key);
    if (ok) {
      return;
    }
  }
  currentChatId.value = generateChatId();
  currentSessionKey.value = `gui:${currentChatId.value}`;
  messages.value = [
    {
      id: generateMessageId(),
      role: 'agent',
      content: t('app.welcome'),
      timestamp: Date.now(),
      emotion: 'happy',
    },
  ];
}

/** @returns true when history was applied to the message list */
async function loadSession(sessionKey: string): Promise<boolean> {
  if (!isTauri()) return false;
  const chatId = extractChatId(sessionKey);

  try {
    let sessionHistory = readSessionFromCache(sessionKey);
    if (!sessionHistory && chatId && chatId !== sessionKey) {
      sessionHistory = readSessionFromCache(chatId);
    }
    if (!sessionHistory) {
      sessionHistory = await withTimeout(
        invoke<BackendSessionHistory | null>("get_session_history", { chatId: sessionKey }),
        SESSION_LOAD_TIMEOUT_MS,
        `get_session_history(${sessionKey})`
      );
      if (sessionHistory && Array.isArray(sessionHistory.messages)) {
        writeSessionToCache(sessionHistory);
      }
    }

    if (!sessionHistory || !Array.isArray(sessionHistory.messages)) {
      messages.value.push({
        id: generateMessageId(),
        role: 'system',
        content: `${t('app.errorPrefix')}Session not found`,
        timestamp: Date.now()
      });
      return false;
    }

    currentChatId.value = extractChatId(sessionHistory.key) || chatId;
    currentSessionKey.value = sessionHistory.key || sessionKey;
    syncPlanRuntime(null);

    const historicalToolNames = buildHistoricalToolNameIndex(sessionHistory.messages);
    const newMessages: Message[] = sessionHistory.messages
      .map((msg) => mapBackendMessageToUi(
        msg,
        msg.tool_call_id ? historicalToolNames.get(msg.tool_call_id) : undefined,
      ))
      .filter((msg): msg is Message => msg !== null);

    if (newMessages.length > 0) {
      messages.value = newMessages;
    } else {
      messages.value.push({
        id: generateMessageId(),
        role: 'system',
        content: `${t('app.errorPrefix')}Session has no displayable messages`,
        timestamp: Date.now()
      });
      return false;
    }

    const selectedSession = sessions.value.find((session) => session.session_key === currentSessionKey.value);
    if (!selectedSession) {
      upsertSession({
        session_key: currentSessionKey.value,
        chat_id: currentChatId.value,
        snippet: compactPreview(newMessages[newMessages.length - 1]?.content || currentChatId.value, 120),
        last_message: compactPreview(newMessages[newMessages.length - 1]?.content || '', 120) || undefined,
        timestamp: Date.now(),
        title: fallbackSessionTitle(newMessages.find((msg) => msg.role === 'user')?.content),
        message_count: newMessages.filter((msg) => ['user', 'agent', 'tool'].includes(msg.role) && msg.content.trim()).length,
        title_generated: false,
        title_manually_set: false,
        pinned: false,
      });
    } else {
      upsertSession({
        ...selectedSession,
        snippet: compactPreview(newMessages[newMessages.length - 1]?.content || selectedSession.snippet, 120),
        last_message: compactPreview(newMessages[newMessages.length - 1]?.content || selectedSession.last_message || '', 120) || undefined,
        timestamp: Date.now(),
        message_count: newMessages.filter((msg) => ['user', 'agent', 'tool'].includes(msg.role) && msg.content.trim()).length,
      });
    }
    await restoreActivePlanRuntime();
    return true;
  } catch (e) {
    console.error("Failed to load session history:", e);
    messages.value.push({ 
      id: generateMessageId(),
      role: 'system', 
      content: t('app.errorPrefix') + e, 
      timestamp: Date.now() 
    });
    return false;
  }
}

async function deleteSession(sessionKey: string) {
  if (!isTauri()) return;
  if (!(await appConfirm(t('chat.confirmDeleteSession')))) return;
  const chatId = extractChatId(sessionKey);
  const wasCurrent = sessionKey === currentSessionKey.value || chatId === currentChatId.value;
  let deleteFailed = false;
  try {
    await invoke('delete_session', { chatId: sessionKey });
  } catch (e) {
    deleteFailed = true;
    console.error('Failed to delete session:', e);
    messages.value.push({
      id: generateMessageId(),
      role: 'system',
      content: `${t('app.errorPrefix')}${e}`,
      timestamp: Date.now(),
    });
  }
  const keysToRemove = new Set<string>([
    ...getSessionCacheKeys(sessionKey),
    ...getSessionCacheKeys(chatId),
  ]);
  for (const key of keysToRemove) {
    try {
      localStorage.removeItem(key);
    } catch (_) {}
  }
  locallyDeletedSessionKeys.value.add(sessionKey);
  sessions.value = sessions.value.filter((session) => session.session_key !== sessionKey);
  await refreshSessions();
  if (wasCurrent) {
    clearMessages();
  }
  if (deleteFailed) {
    messages.value.push({
      id: generateMessageId(),
      role: 'system',
      content: 'Delete failed on backend; removed locally for this run.',
      timestamp: Date.now(),
    });
  }
}

async function saveConfig(newConfig: typeof config.value) {
  try {
    console.log('[App] Saving config:', { 
        ...newConfig, 
        apiKey: newConfig.apiKey ? `${newConfig.apiKey.substring(0, 8)}...` : 'undefined' 
    });

    if (!isTauri()) {
        console.warn('Running in browser, mocking saveConfig');
        showAppToast("保存成功");
        return;
    }

    const rawConfig = await invoke<string>('load_config');
    const parsed = JSON.parse(rawConfig) as RawConfigShape;
    patchProviderConfigInRaw(parsed, newConfig);

    await invoke('save_config', {
      raw: JSON.stringify(parsed, null, 2),
    });

    await invoke("update_config", {
      apiBase: newConfig.apiBase || null,
      apiKey: newConfig.apiKey || null,
      provider: newConfig.provider || null,
      model: newConfig.model || null
    });

    const runtimeConfig = await getRuntimeConfig();
    providerConfigs.value = extractProviderConfigsFromRaw(parsed);
    config.value = {
      provider: runtimeConfig.provider || newConfig.provider,
      apiBase: runtimeConfig.api_base || newConfig.apiBase,
      apiKey: newConfig.apiKey,
      model: runtimeConfig.model || newConfig.model,
    };
    
    showAppToast("保存成功");
  } catch (error) {
    await appAlert(t('app.configUpdateError', { error }));
    throw error;
  }
}

async function saveToolsConfig(newToolsConfig: typeof toolsConfig.value) {
  try {
    toolsConfig.value = JSON.parse(JSON.stringify(newToolsConfig));
    if (!isTauri()) {
      showAppToast("保存成功");
      return;
    }

    await invoke("update_tools_config", {
      tools: newToolsConfig
    });

    showAppToast("保存成功");
  } catch (error) {
    await appAlert(t('app.configUpdateError', { error }));
    throw error;
  }
}

async function saveChannelConfig(channelName: string, channelConfig: Record<string, unknown>) {
  try {
    if (!isTauri()) {
      showAppToast("保存成功");
      return;
    }

    await invoke('update_channel', {
      name: channelName,
      enabled: Boolean(channelConfig.enabled),
      config: channelConfig,
    });

    showAppToast("保存成功");
  } catch (error) {
    await appAlert(t('app.configUpdateError', { error }));
    throw error;
  }
}

async function handleWelcomeDone(payload: WelcomeDonePayload) {
  try {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(WELCOME_STORAGE_KEY, '1');
    }
  } catch (_) {
    /* ignore */
  }
  showWelcomeWizard.value = false;

  if (!payload.skipped) {
    const dk = payload.deepseekApiKey.trim();
    const bk = payload.bochaApiKey.trim();
    if (dk) {
      await saveConfig(buildWelcomeDeepSeekConfig(config.value, dk));
    }
    if (bk) {
      const nextTools = JSON.parse(JSON.stringify(toolsConfig.value)) as typeof toolsConfig.value;
      nextTools.web.search.api_key = bk;
      await saveToolsConfig(nextTools);
    }
  }

  await nextTick();
  const shell = normalModeRef.value as null | {
    openSettingsTab: (view: 'providers' | 'network') => void;
    openConsole: () => void;
  };
  if (!shell) return;
  if (payload.navigate === 'providers') {
    shell.openSettingsTab('providers');
  } else if (payload.navigate === 'network') {
    shell.openSettingsTab('network');
  } else if (payload.navigate === 'console') {
    shell.openConsole();
  }
}

async function checkHealth() {
  if (!isTauri()) return;
  
  try {
    const isHealthy = await invoke<boolean>("check_health");
    const recovered = isHealthy && connectionStatus.value !== 'connected';
    connectionStatus.value = isHealthy ? 'connected' : 'error';
    // The gateway can become ready after the GUI has already completed its one-time
    // startup load. Reload sessions on recovery so a failed initial request does not
    // leave the sidebar permanently empty.
    if (recovered) {
      await refreshSessions();
    }
  } catch (e) {
    console.error("Health check failed:", e);
    connectionStatus.value = 'error';
  }
}

onMounted(async () => {
  const markSplashComplete = () => {
    if (isTauri()) {
      invoke('set_splash_complete', { task: 'frontend' }).catch((e) =>
        console.warn('set_splash_complete failed:', e)
      );
    }
  };

  if (!isTauri()) {
      console.log("Running in browser mode - Tauri listeners skipped");
      return;
  }

  try {
    unlisteners.push(await listen<unknown>('approval-event', (event) => {
      if (isApprovalEventView(event.payload)) void handleUnifiedApprovalEvent(event.payload);
    }));
    unlisteners.push(await listen('approval-stream-connected', () => void refreshUnifiedApprovals()));
    try {
      await withTimeout(
        invoke("start_background_stream"),
        STARTUP_TASK_TIMEOUT_MS,
        "start_background_stream"
      );
    } catch (e) {
      console.warn("Failed to start background stream:", e);
    }
    try {
      await withTimeout(
        invoke('start_approval_stream', { initialCursor: null }),
        STARTUP_TASK_TIMEOUT_MS,
        'start_approval_stream',
      );
      await refreshUnifiedApprovals();
    } catch (e) {
      approvalCenterError.value = approvalActionMessage(e);
      console.warn('Failed to start unified approval stream:', e);
    }
    askUserPollTimer = setInterval(() => void listAskUserQuestions(), 2000);
    await listAskUserQuestions();

    try {
      const [runtimeConfig, rawConfig, status] = await Promise.allSettled([
        withTimeout(getRuntimeConfig(), STARTUP_TASK_TIMEOUT_MS, "getRuntimeConfig"),
        withTimeout(invoke<string>("load_config"), STARTUP_TASK_TIMEOUT_MS, "load_config"),
        withTimeout(getConfigStatus(), STARTUP_TASK_TIMEOUT_MS, "getConfigStatus"),
      ]);
      if (
        runtimeConfig.status !== 'fulfilled' ||
        rawConfig.status !== 'fulfilled' ||
        status.status !== 'fulfilled'
      ) {
        throw new Error(
          [
            runtimeConfig.status === 'rejected' ? `runtime=${runtimeConfig.reason}` : null,
            rawConfig.status === 'rejected' ? `config=${rawConfig.reason}` : null,
            status.status === 'rejected' ? `status=${status.reason}` : null,
          ]
            .filter(Boolean)
            .join('; ')
        );
      }
      const parsed = JSON.parse(rawConfig.value) as RawConfigShape;
      const provider =
        runtimeConfig.value.provider ||
        parsed.agents?.defaults?.provider ||
        status.value.default_provider ||
        config.value.provider;
      const providerConfig = extractProviderConfigFromRaw(parsed, provider);
      providerConfigs.value = extractProviderConfigsFromRaw(parsed);
      config.value = {
        provider,
        apiBase: runtimeConfig.value.api_base || providerConfig?.api_base || config.value.apiBase,
        apiKey: providerConfig?.api_key || "",
        model:
          runtimeConfig.value.model ||
          parsed.agents?.defaults?.model ||
          status.value.default_model ||
          config.value.model,
      };
      syncCurrentConfigToSavedModels(config.value);
    } catch (e) {
      console.warn("Failed to load runtime config:", e);
    }

    try {
      const fetchedTools = await withTimeout(
        invoke<typeof toolsConfig.value>("get_tools_config"),
        STARTUP_TASK_TIMEOUT_MS,
        "get_tools_config"
      );
      toolsConfig.value = fetchedTools;
    } catch (e) {
      console.warn("Failed to load tools config:", e);
    }

    try {
      if (typeof localStorage !== 'undefined' && !localStorage.getItem(WELCOME_STORAGE_KEY)) {
        showWelcomeWizard.value = true;
      }
    } catch (_) {
      /* ignore */
    }

    // Initial health check and polling
    await checkHealth();
    const healthInterval = setInterval(checkHealth, 5000);

    // Fetch sessions and reopen the latest GUI chat (not a fresh random chat id)
    await refreshWorkspace();
    await refreshSessions();
    await restoreLatestGuiChatOnStartup();
    await restoreActivePlanRuntime();

    // Register cleanup
    onUnmounted(() => {
      clearInterval(healthInterval);
    });

    // Listen for streaming text delta
      unlisteners.push(await listen<StreamTextPayload>("agent-response-delta", (event) => {
      if (event.payload.request_id !== activeStreamRequestId.value) {
        return;
      }
      const lastMsg = messages.value[messages.value.length - 1];
      if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
        lastMsg.content += event.payload.data;
      }
    }));

    // Listen for reasoning delta
    unlisteners.push(await listen<StreamTextPayload>("agent-reasoning-delta", (event) => {
    if (event.payload.request_id !== activeStreamRequestId.value) {
      return;
    }
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
      if (!lastMsg.reasoning) {
        lastMsg.reasoning = "";
      }
      lastMsg.reasoning += event.payload.data;
      lastMsg.isThinking = true;
    }
  }));

  // Listen for completion
  unlisteners.push(await listen<StreamTextPayload>("agent-response-complete", (event) => {
    if (event.payload.request_id !== activeStreamRequestId.value) {
      return;
    }
    suppressNextStopError.value = false;
    const completedIndex = completeLatestStreamingAgent(messages.value, event.payload.data);
    if (completedIndex === -1 && event.payload.data) {
      messages.value.push({
        id: generateMessageId(),
        role: 'agent',
        content: event.payload.data,
        isStreaming: false,
        isThinking: false,
        timestamp: Date.now(),
        emotion: currentEmotion.value,
      });
    }
    isTyping.value = false;
    // Refresh plan state before clearing the stream id so a late
    // plan-report-ready for this request can still be accepted, and so
    // restore can re-hydrate the approval card after the turn.
    void restoreActivePlanRuntime().finally(() => {
      if (activeStreamRequestId.value === event.payload.request_id) {
        activeStreamRequestId.value = null;
      }
    });
    syncCurrentSessionListEntry();
    if (isTauri()) {
      void maybeGenerateCurrentSessionTitle();
    }
  }));

  // Listen for tool usage
  unlisteners.push(await listen<StreamTextPayload>("agent-tool-delta", (event) => {
    if (event.payload.request_id !== activeStreamRequestId.value) {
      return;
    }
    // Optional: show tool usage in UI
  }));

  unlisteners.push(await listen<StreamToolStartPayload>("agent-tool-start", (event) => {
    if (event.payload.request_id !== activeStreamRequestId.value) {
      return;
    }
    closeStreamingPlaceholder(true);

    const payload = event.payload || ({} as StreamToolStartPayload);
    const toolName = payload.name || t('app.unknownTool');
    const toolArgs = payload.args_preview || '';

    messages.value.push({ 
      id: generateMessageId(),
      role: 'tool', 
      content: t('app.toolRunning'), 
      timestamp: Date.now(),
      toolName,
      toolArgs,
      toolStatus: 'running',
      toolCallId: payload.call_id || undefined
    });
    
    syncCurrentSessionListEntry();
  }));

  unlisteners.push(await listen<StreamToolFinishPayload>("agent-tool-end", (event) => {
    if (event.payload.request_id !== activeStreamRequestId.value) {
      return;
    }
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && lastMsg.content === '' && lastMsg.isStreaming) {
      messages.value.pop();
    }

    const payload = event.payload || ({} as StreamToolFinishPayload);

    // Prefer matching by call_id to avoid cross-updates when multiple tools run.
    let toolMsgIndex = -1;
    if (payload.call_id) {
      toolMsgIndex = messages.value.findIndex(
        (msg) => msg.role === 'tool' && msg.toolStatus === 'running' && msg.toolCallId === payload.call_id
      );
    }
    if (toolMsgIndex === -1) {
      for (let i = messages.value.length - 1; i >= 0; i--) {
          if (messages.value[i].role === 'tool' && messages.value[i].toolStatus === 'running') {
              toolMsgIndex = i;
              break;
          }
      }
    }

    if (toolMsgIndex !== -1) {
        const isError = payload.is_error === true || payload.result?.startsWith('Error');
        const existing = messages.value[toolMsgIndex];
        const preserveCard = isChecklistCardContent(existing.content);
        existing.toolStatus = isError ? 'error' : 'success';
        if (!preserveCard) {
          existing.content = isError ? t('app.toolError') : t('app.toolSuccess');
        }
        if (payload.name) {
          existing.toolName = payload.name;
        }
        existing.toolResult = payload.result || '';
    } else {
        // If no matching start message found, add a new entry.
        messages.value.push({
          id: generateMessageId(),
          role: 'tool',
          content: payload.is_error ? t('app.toolError') : t('app.toolSuccess'),
          timestamp: Date.now(),
          toolName: payload.name || t('app.unknownTool'),
          toolResult: payload.result || '',
          toolStatus: payload.is_error ? 'error' : 'success',
          toolCallId: payload.call_id || undefined
        });
    }

    messages.value.push({
      id: generateMessageId(),
      role: 'agent',
      content: '',
      isStreaming: true,
      timestamp: Date.now(),
      emotion: currentEmotion.value
    });
  }));

  unlisteners.push(await listen<StreamPlanPayload>("agent-plan-todo-created", (event) => {
    if (event.payload.request_id !== activeStreamRequestId.value) return;
    syncPlanRuntime(event.payload.data.plan);
  }));

  unlisteners.push(await listen<StreamPlanPayload>("agent-plan-todo-updated", (event) => {
    if (event.payload.request_id !== activeStreamRequestId.value) return;
    syncPlanRuntime(event.payload.data.plan);
  }));

  unlisteners.push(await listen<StreamPlanPayload>("agent-plan-todo-completed", (event) => {
    if (event.payload.request_id !== activeStreamRequestId.value) return;
    syncPlanRuntime(event.payload.data.plan);
  }));

  unlisteners.push(await listen<StreamPlanPayload>("agent-plan-todo-cancelled", (event) => {
    if (event.payload.request_id !== activeStreamRequestId.value) return;
    syncPlanRuntime(event.payload.data.plan);
  }));

  unlisteners.push(await listen<StreamPlanPayload>("agent-plan-ready", (event) => {
    if (event.payload.request_id !== activeStreamRequestId.value) return;
    syncPlanRuntime(event.payload.data.plan);
  }));

  unlisteners.push(await listen<StreamJsonPayload>("agent-plan-report-ready", (event) => {
    // Accept while the stream is active. Also accept a late event after
    // complete if the payload is a valid report (restore may race).
    const active = activeStreamRequestId.value;
    if (active && event.payload.request_id !== active) return;
    const plan = planRuntimeFromReportPayload(event.payload.data);
    if (plan) {
      const root = event.payload.data as { report?: { report?: { session_key?: unknown } } };
      const sessionKey = root.report?.report?.session_key;
      pendingApprovalSessionKey.value = typeof sessionKey === 'string' ? sessionKey : null;
      syncPlanRuntime(plan);
    }
  }));

  // Listen for normal-chat TODO/checklist updates from update_plan.
  unlisteners.push(await listen<StreamTurnPlanPayload>("agent-turn-plan-updated", (event) => {
    if (event.payload.request_id !== activeStreamRequestId.value) {
      return;
    }
    const { explanation, plan } = event.payload.data;
    const card = {
      id: generateMessageId(),
      kind: 'checklist' as const,
      explanation: explanation || undefined,
      plan_items: plan || [],
      status: 'updated',
      title: 'Task Checklist',
      summary: '',
      body_markdown: '',
      actions: [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };

    // Update the current turn's tool row even if transport delivery races
    // with tool_finish. Never reuse a checklist from an earlier user turn.
    const existingIndex = findCurrentTurnUpdatePlanToolIndex(messages.value);
    if (existingIndex !== -1) {
      messages.value[existingIndex].content = JSON.stringify(card);
    } else {
      const checklistMessage: Message = {
        id: generateMessageId(),
        role: 'tool',
        content: JSON.stringify(card),
        timestamp: Date.now(),
        toolName: 'update_plan',
        toolStatus: 'success',
      };
      const streamingIndex = findLatestStreamingAgentIndex(messages.value);
      if (streamingIndex === -1) {
        messages.value.push(checklistMessage);
      } else {
        messages.value.splice(streamingIndex, 0, checklistMessage);
      }
    }
    syncCurrentSessionListEntry();
  }));

  // Listen for errors
  unlisteners.push(await listen<unknown>("agent-error", (event) => {
    const payload = event.payload;
    const isStreamPayload =
      typeof payload === 'object'
      && payload !== null
      && 'request_id' in payload
      && 'data' in payload;
    const requestId = isStreamPayload ? String((payload as StreamTextPayload).request_id) : null;
    const errorMessage = isStreamPayload
      ? String((payload as StreamTextPayload).data)
      : String(payload ?? '');

    if (requestId && requestId !== activeStreamRequestId.value) {
      return;
    }

    if (suppressNextStopError.value && errorMessage === "Generation stopped by user.") {
      suppressNextStopError.value = false;
      if (requestId && requestId === activeStreamRequestId.value) {
        activeStreamRequestId.value = null;
      }
      return;
    }
    
    // Check if the last message is a duplicate error message to debounce
    if (messages.value.length > 0) {
        const lastMsg = messages.value[messages.value.length - 1];
        
        // Remove streaming placeholder if exists
        if (lastMsg.role === 'agent' && lastMsg.isStreaming) {
            messages.value.pop();
        }
        
        // Re-check last message after pop
        if (messages.value.length > 0) {
            const newLastMsg = messages.value[messages.value.length - 1];
            if (newLastMsg.role === 'system' && newLastMsg.content === `${t('app.errorPrefix')}${errorMessage}`) {
                return; // Skip duplicate
            }
        }
    }

    messages.value.push({
      id: generateMessageId(),
      role: 'system', 
      content: `${t('app.errorPrefix')}${errorMessage}`, 
      timestamp: Date.now() 
    });
    if (requestId && requestId === activeStreamRequestId.value) {
      isTyping.value = false;
      activeStreamRequestId.value = null;
    }
    syncCurrentSessionListEntry();
  }));

  // Listen for provider retry progress (shown on the streaming message)
  unlisteners.push(await listen<StreamRetryPayload>("agent-provider-retry", (event) => {
    if (event.payload.request_id !== activeStreamRequestId.value) {
      return;
    }
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
      lastMsg.retryStatus = {
        attempt: event.payload.attempt,
        maxRetries: event.payload.max_retries,
        model: event.payload.model,
      };
    }
  }));

  // Listen for provider stall hints (long idle without a terminal event)
  unlisteners.push(await listen<StreamStalledPayload>("agent-provider-stalled", (event) => {
    if (event.payload.request_id !== activeStreamRequestId.value) {
      return;
    }
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
      lastMsg.stalled = true;
    }
  }));

  // Listen for automatic/reactive context compaction progress.
  unlisteners.push(await listen<StreamContextCompactionPayload>('agent-context-compaction', (event) => {
    const payload = event.payload?.data;
    if (!payload || payload.session_id !== currentSessionKey.value) return;
    if (payload.trigger !== 'auto' && payload.trigger !== 'reactive') return;
    if (payload.phase !== 'started' && payload.phase !== 'completed' && payload.phase !== 'failed') return;
    compactionStatus.value = {
      trigger: payload.trigger,
      phase: payload.phase,
      summary: payload.summary,
    };
  }));

  // Listen for external hook messages
  unlisteners.push(await listen<string>("external-message", (event) => {
    messages.value.push({ 
      id: generateMessageId(),
      role: 'system', 
      content: t('app.hookMessage', { message: event.payload }), 
      timestamp: Date.now() 
    });
  }));

  // Listen for background responses (e.g. scheduled cron executions)
  unlisteners.push(await listen<string>("agent-background-response", (event) => {
    messages.value.push({
      id: generateMessageId(),
      role: 'agent',
      content: event.payload,
      timestamp: Date.now(),
      emotion: currentEmotion.value
    });
  }));
  } catch (e) {
    console.error("App initialization error:", e);
  } finally {
    markSplashComplete();
  }
});

onUnmounted(() => {
  if (askUserPollTimer) clearInterval(askUserPollTimer);
  unlisteners.forEach(fn => fn());
});
</script>

<template>
  <div class="w-screen h-screen overflow-hidden bg-transparent relative">
    <WelcomeWizard
      :open="showWelcomeWizard"
      :config="config"
      :tools-config="toolsConfig"
      @done="handleWelcomeDone"
    />
    <PersonaSetupGate v-if="!showWelcomeWizard" />
    <NormalMode
      ref="normalModeRef"
      :messages="messages"
      :is-typing="isTyping"
      :connection-status="connectionStatus"
      :current-emotion="currentEmotion"
      :config="config"
      :provider-configs="providerConfigs"
      :tools-config="toolsConfig"
      :saved-models="savedModels"
      :sessions="sessions"
      :chat-display-prefs="chatDisplayPrefs"
      :current-session-key="currentSessionKey"
      :workspace="workspace"
      :workspace-state="workspaceState"
      :workspace-error="workspaceError"
      :refresh-workspace="refreshWorkspace"
      :switch-workspace="switchWorkspaceAction"
      :switch-blocked-reason="workspaceSwitchBlockedReason"
      :switching="workspaceSwitching"
      :active-plan-runtime="activePlanRuntime"
      :pending-approval-plan="pendingApprovalPlan"
      :executing-plan="executingPlan"
      :approving-plan="approvingPlan"
      :plan-execution-error="currentPlanContinuationError"
      :approval-center-open="approvalCenterOpen"
      :approval-pending-count="approvalPendingCount"
      :ask-user-questions="pendingQuestions"
      :compaction-status="compactionStatus"
      :save-config-action="saveConfig"
      :save-tools-config-action="saveToolsConfig"
      :save-channel-config-action="saveChannelConfig"
      @send="sendMessage"
      @approve-plan="approvePlanExecution"
      @resume-plan="resumePlanExecution"
      @revoke-plan="revokePlanExecution"
      @refresh-plan="restoreActivePlanRuntime"
      @refresh-sessions="refreshSessions"
      @clear="clearMessages"
      @stop="stopMessage"
      @regenerate="regenerateMessage"
      @update-saved-models="updateSavedModels"
      @save-chat-display-prefs="updateChatDisplayPrefs"
      @load-session="loadSession"
      @delete-session="deleteSession"
      @update:approval-center-open="onApprovalCenterOpenChange"
      @answer-ask-user="answerAskUserQuestion"
      @cancel-ask-user="cancelAskUserQuestion"
    />
    <ApprovalCenterDrawer
      v-model:open="approvalCenterOpen"
      :approvals="unifiedApprovals"
      :details="approvalDetails"
      :loading="approvalCenterLoading"
      :error="approvalCenterError"
      :submitting-ids="unifiedSubmittingIds"
      :outcome-unknown-ids="unifiedOutcomeUnknownIds"
      :action-errors="unifiedActionErrors"
      @refresh="refreshUnifiedApprovals"
      @inspect="refreshUnifiedApproval"
      @decide="decideUnifiedApproval"
      @cancel="cancelUnifiedApproval"
      @edit="editUnifiedApproval"
      @refresh-one="refreshUnifiedApproval"
    />
  </div>
</template>
