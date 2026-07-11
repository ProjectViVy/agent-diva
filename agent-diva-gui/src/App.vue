<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import NormalMode from "./components/NormalMode.vue";
import WelcomeWizard from "./components/WelcomeWizard.vue";
import { appAlert, appConfirm } from "./utils/appDialog";
import { showAppToast } from "./utils/appToast";
import { useI18n } from "vue-i18n";
import {
  approveActivePlanExecution,
  getConfigStatus,
  getRuntimeConfig,
  returnActivePlanToDraft,
  FileAttachmentDto,
} from "./api/desktop";
import {
  planReportValidationIssues,
  type PlanDetail,
  type PlanRuntimeState,
  type PlanSnapshotMetadata,
  type PlanStreamEvent,
} from "./api/planning";
import type { ToolsConfigShape } from "./types/toolsConfig";
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

const { t } = useI18n();

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
  rawMeta?: Record<string, unknown>;
  fromHistory?: boolean;
  attachments?: string[];
  planSnapshot?: PlanRuntimeState;
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
}
interface ChatDisplayPrefs {
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
const activeStreamRequestId = ref<string | null>(null);
const activePlanRuntime = ref<PlanRuntimeState | null>(null);
const pendingApprovalPlan = ref<PlanRuntimeState | null>(null);
const executingPlan = ref<PlanRuntimeState | null>(null);
const approvingPlan = ref(false);
const locallyDeletedSessionKeys = ref<Set<string>>(new Set());
const titleGenerationInFlight = ref<Set<string>>(new Set());

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
  mentle: {
    enabled: false,
    mode: 'off',
    allowed_tools: [],
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

const unlisteners: UnlistenFn[] = [];

const showWelcomeWizard = ref(false);
const normalModeRef = ref<InstanceType<typeof NormalMode> | null>(null);

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

function extractToolName(
  role: string,
  name?: string | null,
  toolCalls?: serdeJsonValue[] | null
): string | undefined {
  if (name && name.trim()) return name.trim();
  if (!toolCalls || toolCalls.length === 0) return role === 'tool' ? 'tool' : undefined;
  const firstCall = toolCalls[0];
  const maybeFn = firstCall?.function as Record<string, unknown> | undefined;
  const fnName = maybeFn?.name;
  if (typeof fnName === 'string' && fnName.trim()) return fnName.trim();
  return role === 'tool' ? 'tool' : undefined;
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

function mapBackendMessageToUi(msg: BackendChatMessage): Message | null {
  const mappedRole = mapSessionRole(msg.role);
  if (!mappedRole) {
    return null;
  }

  const toolName = extractToolName(msg.role, msg.name, msg.tool_calls);
  const toolArgs = extractToolArgs(msg.tool_calls);
  const rawMeta = buildRawMeta(msg);
  const planMetadata = msg.metadata as Partial<PlanSnapshotMetadata> | null | undefined;
  const planSnapshot = planMetadata?.kind === 'plan_snapshot' && planMetadata.plan
    ? planMetadata.plan
    : undefined;
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
    planSnapshot,
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
  const normalized: PlanRuntimeState = {
    ...plan,
    title: sanitizePlanText(plan.title) || plan.title || 'Plan',
    goal: sanitizePlanText(plan.goal) || plan.goal,
    markdown: plan.markdown ? sanitizePlanText(plan.markdown) : plan.markdown,
    summary: sanitizePlanText(plan.summary) || plan.summary,
    strategy: plan.strategy == null ? null : sanitizePlanText(plan.strategy) || plan.strategy,
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
  return value.replace(/\uFFFD/g, '').trim();
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
  const markdown = sanitizePlanText(revisionMeta.markdown || (detail.markdown as string) || '');
  if (!id || !markdown) return null;

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

async function approvePlanExecution(payload: { contextPolicy: 'retain' | 'compact' | 'clear' }) {
  if (approvingPlan.value) return;
  approvingPlan.value = true;
  try {
    const pending = pendingApprovalPlan.value;
    if (pending?.revision == null) throw new Error('Plan revision is unavailable; refresh before approving.');
    const result = await approveActivePlanExecution({
      expected_revision: pending.revision,
      todo_policy: 'Optional',
      materialize_todos: false,
      context_policy: payload.contextPolicy,
    });
    syncPlanRuntime(result.plan);
    messages.value.push({
      id: generateMessageId(),
      role: 'system',
      content: `计划已批准：revision ${result.receipt.revision}，审批时间 ${result.receipt.approved_at}。已记录“${payload.contextPolicy}”上下文策略（当前仅为 GUI 原型，未传给运行时）。TODO 将由 agent 在执行时按需创建。`,
      timestamp: Date.now(),
    });
  } catch (error) {
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
    const plan = await invoke<PlanDetail | null>('get_active_plan');
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
    const reopened = await returnActivePlanToDraft();
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

async function sendMessage(content: string, attachments?: FileAttachmentDto[], mode: ExecMode = 'agent') {
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

    await invoke("send_message", {
      message: content,
      channel: currentChannel.value,
      chatId: currentChatId.value,
      attachments: attachmentFileIds,
      mode,
      streamRequestId,
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
      const lastMsg = messages.value[messages.value.length - 1];
      if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
        lastMsg.isStreaming = false;
        lastMsg.isThinking = false;
      }
      isTyping.value = false;
      messages.value.push({
        id: generateMessageId(),
        role: 'system',
        content: `[Mock] ${t('app.stopped')}`,
        timestamp: Date.now()
      });
      return;
    }

    await invoke("stop_generation", {
      channel: currentChannel.value,
      chatId: currentChatId.value,
    });
    suppressNextStopError.value = true;
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
      lastMsg.isStreaming = false;
      lastMsg.isThinking = false;
    }
    isTyping.value = false;
    messages.value.push({
      id: generateMessageId(),
      role: 'system',
      content: t('app.stopRequested'),
      timestamp: Date.now()
    });
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

async function refreshSessions() {
  if (!isTauri()) return;
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
        } satisfies SessionInfo;
      });
      sessions.value = mapped
        .filter((session) => !locallyDeletedSessionKeys.value.has(session.session_key))
        .sort((a, b) => b.timestamp - a.timestamp);
    }
  } catch (e) {
    console.error("Failed to fetch sessions:", e);
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

    const newMessages: Message[] = sessionHistory.messages
      .map(mapBackendMessageToUi)
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
    connectionStatus.value = isHealthy ? 'connected' : 'error';
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
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
      if (!lastMsg.content && event.payload.data) {
         lastMsg.content = event.payload.data;
      }
      lastMsg.isStreaming = false;
      lastMsg.isThinking = false;
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
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && (lastMsg.content === '') && lastMsg.isStreaming) {
      messages.value.pop();
    } else if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
      lastMsg.isStreaming = false;
    }

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
    
    // Add placeholder for next agent response
    messages.value.push({
      id: generateMessageId(),
      role: 'agent',
      content: '',
      isStreaming: true, 
      timestamp: Date.now(),
      emotion: currentEmotion.value
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
        messages.value[toolMsgIndex].toolStatus = isError ? 'error' : 'success';
        messages.value[toolMsgIndex].content = isError ? t('app.toolError') : t('app.toolSuccess');
        if (payload.name) {
          messages.value[toolMsgIndex].toolName = payload.name;
        }
        messages.value[toolMsgIndex].toolResult = payload.result || '';
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
      syncPlanRuntime(plan);
    }
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
      :active-plan-runtime="activePlanRuntime"
      :pending-approval-plan="pendingApprovalPlan"
      :executing-plan="executingPlan"
      :approving-plan="approvingPlan"
      :save-config-action="saveConfig"
      :save-tools-config-action="saveToolsConfig"
      :save-channel-config-action="saveChannelConfig"
      @send="sendMessage"
      @approve-plan="approvePlanExecution"
      @revoke-plan="revokePlanExecution"
      @refresh-plan="restoreActivePlanRuntime"
      @clear="clearMessages"
      @stop="stopMessage"
      @regenerate="regenerateMessage"
      @update-saved-models="updateSavedModels"
      @save-chat-display-prefs="updateChatDisplayPrefs"
      @load-session="loadSession"
      @delete-session="deleteSession"
    />
  </div>
</template>
