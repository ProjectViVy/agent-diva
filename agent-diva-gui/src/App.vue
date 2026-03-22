<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import NormalMode from "./components/NormalMode.vue";
import WelcomeWizard from "./components/WelcomeWizard.vue";
import { appAlert, appConfirm } from "./utils/appDialog";
import { useI18n } from "vue-i18n";
import { getConfigStatus, getRuntimeConfig } from "./api/desktop";
import {
  HISTORY_PREFS_KEY,
  SAVED_MODELS_KEY,
  SESSION_CACHE_PREFIX,
  WELCOME_STORAGE_KEY,
} from "./utils/localStorageAgentDiva";

const { t } = useI18n();

interface Message {
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

const SESSION_CACHE_TTL_MS = 30 * 60 * 1000;
const defaultChatDisplayPrefs: ChatDisplayPrefs = {
  autoExpandReasoning: true,
  autoExpandToolDetails: false,
  showRawMetaByDefault: false,
};

const messages = ref<Message[]>([
  {
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
const locallyDeletedSessionKeys = ref<Set<string>>(new Set());

// Config state
const config = ref({
  provider: "deepseek",
  apiBase: "https://api.deepseek.com/v1",
  apiKey: "",
  model: "deepseek-chat"
});

const toolsConfig = ref({
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
  }
});

const savedModels = ref<SavedModel[]>([]);
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

function extractProviderConfigFromRaw(
  parsed: {
    providers?: Record<string, RawProviderConfig> & {
      custom_providers?: Record<string, RawProviderConfig>;
    };
  },
  provider?: string | null
): RawProviderConfig | undefined {
  if (!provider) {
    return undefined;
  }

  return parsed.providers?.[provider] || parsed.providers?.custom_providers?.[provider];
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
  const toolResult = mappedRole === 'tool' ? (msg.content || '') : undefined;
  const toolStatus = mappedRole === 'tool'
    ? (/^error\b/i.test(msg.content || '') ? 'error' : 'success')
    : undefined;

  return {
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

async function sendMessage(content: string) {
  if (!content.trim() || isTyping.value) return;
  if (content.trim() === '/stop') {
    await stopMessage();
    return;
  }

  console.log('[App] Sending message with current config:', {
      ...config.value,
      apiKey: config.value.apiKey ? `${config.value.apiKey.substring(0, 8)}...` : 'undefined'
  });

  const userMsg: Message = { 
    role: 'user', 
    content: content, 
    timestamp: Date.now() 
  };
  messages.value.push(userMsg);
  
  isTyping.value = true;
  suppressNextStopError.value = false;
  closeStreamingPlaceholder(true);
  const streamRequestId = generateStreamRequestId();
  activeStreamRequestId.value = streamRequestId;
  
  // Create a placeholder for the agent response
  messages.value.push({ 
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
            }
        }, 1000);
        return;
    }

    await invoke("send_message", {
      message: content,
      channel: currentChannel.value,
      chatId: currentChatId.value,
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
      role: 'system',
      content: t('app.stopRequested'),
      timestamp: Date.now()
    });
  } catch (error) {
    messages.value.push({
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
  messages.value = [
    {
      role: 'agent',
      content: t('app.cleared'),
      timestamp: Date.now(),
      emotion: 'happy'
    }
  ];
  
  // Refresh sessions list quietly when cleared because it produces a new session file
  if (isTauri()) {
    setTimeout(refreshSessions, 1000);
  }
};

async function refreshSessions() {
  if (!isTauri()) return;
  try {
    const fetched = await invoke<BackendSessionInfo[]>("get_sessions");
    if (fetched && Array.isArray(fetched)) {
      const mapped = fetched.map((session) => {
        const chatId = extractChatId(session.key);
        return {
          session_key: session.key,
          chat_id: chatId,
          snippet: chatId || session.key || '...',
          timestamp: parseSessionTimestamp(session.updated_at, session.created_at),
        };
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
      sessionHistory = await invoke<BackendSessionHistory | null>("get_session_history", { chatId: sessionKey });
      if (sessionHistory && Array.isArray(sessionHistory.messages)) {
        writeSessionToCache(sessionHistory);
      }
    }

    if (!sessionHistory || !Array.isArray(sessionHistory.messages)) {
      messages.value.push({
        role: 'system',
        content: `${t('app.errorPrefix')}Session not found`,
        timestamp: Date.now()
      });
      return false;
    }

    currentChatId.value = extractChatId(sessionHistory.key) || chatId;
    currentSessionKey.value = sessionHistory.key || sessionKey;

    const newMessages: Message[] = sessionHistory.messages
      .map(mapBackendMessageToUi)
      .filter((msg): msg is Message => msg !== null);

    if (newMessages.length > 0) {
      messages.value = newMessages;
    } else {
      messages.value.push({
        role: 'system',
        content: `${t('app.errorPrefix')}Session has no displayable messages`,
        timestamp: Date.now()
      });
      return false;
    }

    const selectedSession = sessions.value.find((session) => session.session_key === currentSessionKey.value);
    if (!selectedSession) {
      sessions.value.unshift({
        session_key: currentSessionKey.value,
        chat_id: currentChatId.value,
        snippet: currentChatId.value,
        timestamp: Date.now(),
      });
    } else {
      selectedSession.timestamp = Date.now();
      sessions.value = [...sessions.value].sort((a, b) => b.timestamp - a.timestamp);
    }
    return true;
  } catch (e) {
    console.error("Failed to load session history:", e);
    messages.value.push({ 
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
        messages.value.push({ 
            role: 'system', 
            content: "[Mock] " + t('app.configUpdated'), 
            timestamp: Date.now() 
        });
        return;
    }

    await invoke("update_config", {
      apiBase: newConfig.apiBase || null,
      apiKey: newConfig.apiKey || null,
      provider: newConfig.provider || null,
      model: newConfig.model || null
    });

    const runtimeConfig = await getRuntimeConfig();
    config.value = {
      provider: runtimeConfig.provider || newConfig.provider,
      apiBase: runtimeConfig.api_base || newConfig.apiBase,
      apiKey: newConfig.apiKey,
      model: runtimeConfig.model || newConfig.model,
    };
    
    messages.value.push({ 
      role: 'system', 
      content: t('app.configUpdated'), 
      timestamp: Date.now() 
    });
  } catch (error) {
    await appAlert(t('app.configUpdateError', { error }));
  }
}

async function saveToolsConfig(newToolsConfig: typeof toolsConfig.value) {
  try {
    toolsConfig.value = JSON.parse(JSON.stringify(newToolsConfig));
    if (!isTauri()) {
      messages.value.push({
        role: 'system',
        content: "[Mock] " + t('app.configUpdated'),
        timestamp: Date.now()
      });
      return;
    }

    await invoke("update_tools_config", {
      tools: newToolsConfig
    });

    messages.value.push({
      role: 'system',
      content: t('app.configUpdated'),
      timestamp: Date.now()
    });
  } catch (error) {
    await appAlert(t('app.configUpdateError', { error }));
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
      await saveConfig({
        ...config.value,
        apiKey: dk,
      });
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
      await invoke("start_background_stream");
    } catch (e) {
      console.warn("Failed to start background stream:", e);
    }

    try {
      const [runtimeConfig, rawConfig, status] = await Promise.all([
        getRuntimeConfig(),
        invoke<string>("load_config"),
        getConfigStatus(),
      ]);
      const parsed = JSON.parse(rawConfig) as {
        agents?: { defaults?: { provider?: string | null; model?: string | null } };
        providers?: Record<string, RawProviderConfig> & {
          custom_providers?: Record<string, RawProviderConfig>;
        };
      };
      const provider =
        runtimeConfig.provider ||
        parsed.agents?.defaults?.provider ||
        status.default_provider ||
        config.value.provider;
      const providerConfig = extractProviderConfigFromRaw(parsed, provider);
      config.value = {
        provider,
        apiBase: runtimeConfig.api_base || providerConfig?.api_base || config.value.apiBase,
        apiKey: providerConfig?.api_key || "",
        model:
          runtimeConfig.model ||
          parsed.agents?.defaults?.model ||
          status.default_model ||
          config.value.model,
      };
      syncCurrentConfigToSavedModels(config.value);
    } catch (e) {
      console.warn("Failed to load runtime config:", e);
    }

    try {
      const fetchedTools = await invoke<typeof toolsConfig.value>("get_tools_config");
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
      activeStreamRequestId.value = null;
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
      role: 'agent', 
      content: '', 
      isStreaming: true, 
      timestamp: Date.now(),
      emotion: currentEmotion.value
    });
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
      role: 'agent',
      content: '',
      isStreaming: true,
      timestamp: Date.now(),
      emotion: currentEmotion.value
    });
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
      role: 'system', 
      content: `${t('app.errorPrefix')}${errorMessage}`, 
      timestamp: Date.now() 
    });
    if (requestId && requestId === activeStreamRequestId.value) {
      isTyping.value = false;
      activeStreamRequestId.value = null;
    }
  }));

  // Listen for external hook messages
  unlisteners.push(await listen<string>("external-message", (event) => {
    messages.value.push({ 
      role: 'system', 
      content: t('app.hookMessage', { message: event.payload }), 
      timestamp: Date.now() 
    });
  }));

  // Listen for background responses (e.g. scheduled cron executions)
  unlisteners.push(await listen<string>("agent-background-response", (event) => {
    messages.value.push({
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
      :tools-config="toolsConfig"
      :saved-models="savedModels"
      :sessions="sessions"
      :chat-display-prefs="chatDisplayPrefs"
      @send="sendMessage"
      @clear="clearMessages"
      @stop="stopMessage"
      @save-config="saveConfig"
      @save-tools-config="saveToolsConfig"
      @update-saved-models="updateSavedModels"
      @save-chat-display-prefs="updateChatDisplayPrefs"
      @load-session="loadSession"
      @delete-session="deleteSession"
    />
  </div>
</template>
