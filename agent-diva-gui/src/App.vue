<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, watch, nextTick } from "vue";
import type { AskUserQuestionView, CompactionStatus } from './components/ChatView.vue';
import NormalMode from "./components/NormalMode.vue";
import ApprovalCenterDrawer from "./components/ApprovalCenterDrawer.vue";
import WelcomeWizard from "./components/WelcomeWizard.vue";
import { appAlert, appConfirm } from "./utils/appDialog";
import { showAppToast } from "./utils/appToast";
import { useI18n } from "vue-i18n";
import type {
  TurnAttachment,
} from "./api/vivy/contracts";
import {
  loadProviderState,
  saveActiveProvider,
  type ProviderConfigEntry,
} from "./api/settings";
import {
  type PlanRuntimeState,
} from "./api/planning";
import {
  type ApprovalGrant,
  type ApprovalView,
  type UnifiedApprovalApiError,
} from "./api/approvals";
import {
  HISTORY_PREFS_KEY,
  SAVED_MODELS_KEY,
  WELCOME_STORAGE_KEY,
} from "./utils/localStorageAgentDiva";
import {
  DEFAULT_DEEPSEEK_API_BASE,
  DEFAULT_DEEPSEEK_MODEL,
  DEFAULT_DEEPSEEK_PROVIDER,
  buildWelcomeDeepSeekConfig,
} from "./utils/welcomeConfig";
import { vivyClient } from './api/vivy/instance';
import { VivyChatController } from './state/vivy-chat';
import { generateMessageId, type ChatMessage } from './state/chat-message';

const { t } = useI18n();

type Message = ChatMessage;

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
  cleanMode: boolean;
  autoExpandReasoning: boolean;
  autoExpandToolDetails: boolean;
  showRawMetaByDefault: boolean;
}


const STARTUP_TASK_TIMEOUT_MS = 2500;
const defaultChatDisplayPrefs: ChatDisplayPrefs = {
  cleanMode: false,
  autoExpandReasoning: true,
  autoExpandToolDetails: false,
  showRawMetaByDefault: false,
};

const messages = ref<Message[]>([
  {
    id: generateMessageId(),
    role: 'agent',
    content: t('app.welcome'),
    timestamp: Date.now(),
    emotion: 'happy'
  }
]);
const isTyping = ref(false);
const connectionStatus = ref<'connected' | 'error' | 'connecting'>('connecting');
const currentEmotion = ref('happy');
const currentSessionKey = ref('');
const compactionStatus = ref<CompactionStatus | null>(null);
const activePlanRuntime = ref<PlanRuntimeState | null>(null);
const pendingApprovalPlan = ref<PlanRuntimeState | null>(null);
const executingPlan = ref<PlanRuntimeState | null>(null);
const approvingPlan = ref(false);
const planContinuationError = ref<string | null>(null);

/** DN-2: VIVY chat controller — the single orchestration authority. */
const vivyChat = new VivyChatController(vivyClient);
let detachVivySync: (() => void) | null = null;

const currentPlanContinuationError = computed(() => planContinuationError.value);

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

const showWelcomeWizard = ref(false);
const normalModeRef = ref<InstanceType<typeof NormalMode> | null>(null);

const approvalPendingCount = computed(() =>
  unifiedApprovals.value.filter((approval) => approval.status === 'pending').length,
);

const pendingQuestions = ref<AskUserQuestionView[]>([]);
const askUserSubmittingIds = ref<string[]>([]);
const askUserError = ref<string | null>(null);

async function answerAskUserQuestion(payload: { question_id: string; selected_index: number | null; other_text: string | null }) {
  if (!isTauri()) return;
  askUserSubmittingIds.value = [...askUserSubmittingIds.value, payload.question_id];
  askUserError.value = null;
  // VIVY questions are free-text: the answer is the typed text.
  const answer = payload.other_text ?? '';
  try {
    await vivyChat.answerQuestion(payload.question_id, answer);
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
    await vivyChat.cancelQuestion(questionId);
  } catch (error) {
    askUserError.value = String(error);
  } finally {
    askUserSubmittingIds.value = askUserSubmittingIds.value.filter((id) => id !== questionId);
  }
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
    await vivyChat.refreshInteractions();
  } catch (error) {
    approvalCenterError.value = approvalActionMessage(error);
  } finally {
    approvalCenterLoading.value = false;
  }
}

async function refreshUnifiedApproval(requestId: string) {
  if (!isTauri()) return;
  try {
    await vivyChat.refreshInteractions();
    const detail = vivyChat.approvals().find((item) => item.request_id === requestId);
    if (!detail) {
      // The authoritative list no longer carries it — decided elsewhere.
      unifiedOutcomeUnknownIds.value = unifiedOutcomeUnknownIds.value.filter((id) => id !== requestId);
      return;
    }
    approvalDetails.value = { ...approvalDetails.value, [requestId]: detail };
    unifiedOutcomeUnknownIds.value = unifiedOutcomeUnknownIds.value.filter((id) => id !== requestId);
    const errors = { ...unifiedActionErrors.value };
    delete errors[requestId];
    unifiedActionErrors.value = errors;
  } catch (error) {
    unifiedActionErrors.value = { ...unifiedActionErrors.value, [requestId]: approvalActionMessage(error) };
  }
}

async function decideUnifiedApproval(payload: { approval: ApprovalView; decision: 'allow' | 'deny'; grant: ApprovalGrant }) {
  const { approval } = payload;
  if (unifiedSubmittingIds.value.includes(approval.request_id)) return;
  unifiedSubmittingIds.value = [...unifiedSubmittingIds.value, approval.request_id];
  try {
    // Decisions bind to the backend review id; VIVY has no plan-domain
    // review item (plan reviews live on session/work).
    await vivyChat.decideApproval(approval.request_id, payload.decision === 'allow');
    if (payload.decision === 'allow' && approvalDrawerAutoOpened.value) {
      approvalDrawerAutoOpened.value = false;
      approvalCenterOpen.value = false;
    }
  } finally {
    unifiedSubmittingIds.value = unifiedSubmittingIds.value.filter((id) => id !== approval.request_id);
    syncFromController();
  }
}

async function cancelUnifiedApproval(approval: ApprovalView) {
  if (unifiedSubmittingIds.value.includes(approval.request_id)) return;
  unifiedSubmittingIds.value = [...unifiedSubmittingIds.value, approval.request_id];
  try {
    await vivyChat.cancelApproval(approval.request_id);
  } finally {
    unifiedSubmittingIds.value = unifiedSubmittingIds.value.filter((id) => id !== approval.request_id);
    syncFromController();
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


type WelcomeDonePayload = {
  skipped: boolean;
  deepseekApiKey: string;
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

function updateSavedModels(models: SavedModel[]) {
  savedModels.value = models;
  try {
    localStorage.setItem(SAVED_MODELS_KEY, JSON.stringify(models));
  } catch (_) {
    /* ignore */
  }
}

function updateChatDisplayPrefs(prefs: ChatDisplayPrefs) {
  chatDisplayPrefs.value = {
    ...defaultChatDisplayPrefs,
    ...prefs,
  };
  try {
    localStorage.setItem(HISTORY_PREFS_KEY, JSON.stringify(chatDisplayPrefs.value));
  } catch (_) {
    /* ignore */
  }
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

function pushSystemNotice(content: string) {
  messages.value.push({
    id: generateMessageId(),
    role: 'system',
    content,
    timestamp: Date.now(),
  });
}

async function approvePlanExecution() {
  if (approvingPlan.value) return;
  approvingPlan.value = true;
  try {
    // plan/decide 'execute_once' approves the pending plan submission.
    await vivyChat.decidePlan('execute_once');
    planContinuationError.value = null;
  } catch (error) {
    planContinuationError.value = error instanceof Error ? error.message : String(error);
    pushSystemNotice(`${t('app.errorPrefix')}${error}`);
  } finally {
    approvingPlan.value = false;
  }
}

async function resumePlanExecution() {
  if (!planContinuationError.value || approvingPlan.value || isTyping.value) return;
  approvingPlan.value = true;
  planContinuationError.value = null;
  try {
    // goal/resume: restart the paused/failed goal loop.
    await vivyChat.resumeWork();
  } catch (error) {
    planContinuationError.value = error instanceof Error ? error.message : String(error);
    pushSystemNotice(`${t('app.errorPrefix')}${error}`);
  } finally {
    approvingPlan.value = false;
  }
}

async function restoreActivePlanRuntime() {
  if (!isTauri()) return;
  const sessionId = vivyChat.currentSessionId;
  if (!sessionId) return;
  await vivyChat.refreshWork(sessionId).catch((error) => {
    console.warn('Failed to restore plan runtime (session/work):', error);
  });
}

async function revokePlanExecution(feedback = '') {
  const plan = pendingApprovalPlan.value;
  if (!plan) return;
  try {
    // plan/decide 'revise' returns the plan to draft with optional feedback.
    await vivyChat.decidePlan('revise', feedback.trim() || undefined);
  } catch (error) {
    pushSystemNotice(`${t('app.errorPrefix')}${error}`);
  }
}

async function sendMessage(
  content: string,
  attachments?: TurnAttachment[],
  _mode?: string,
  permissionMode?: 'cautious' | 'smart' | 'trusted',
) {
  if (!content.trim()) return;
  if (isTyping.value) return;
  if (content.trim() === '/stop') {
    await stopMessage();
    return;
  }
  if (!isTauri()) return;
  try {
    // DN-2A: the controller validates images/frame size, arms+confirms the
    // preset through session/set_permission, then starts the turn — one
    // serialized mutation lane per session.
    await vivyChat.send(content, {
      ...(attachments?.length ? { attachments } : {}),
      ...(permissionMode ? { preset: permissionMode } : {}),
    });
  } catch (error) {
    pushSystemNotice(`${t('app.errorPrefix')}${error}`);
  }
}

async function regenerateMessage(_messageId: string) {
  if (isTyping.value) return;
  // VIVY has no in-place rewind: resend the last user turn as a new turn.
  const lastUser = [...messages.value].reverse().find(
    (m) => m.role === 'user' && m.content?.trim(),
  );
  if (!lastUser) return;
  await sendMessage(lastUser.content);
}

async function stopMessage() {
  if (!isTyping.value) return;
  try {
    await vivyChat.stop();
  } catch (error) {
    pushSystemNotice(t('app.stopFailed', { error }));
  }
}

const clearMessages = () => {
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
  if (isTauri()) void vivyChat.newSession();
};

async function refreshSessions(): Promise<boolean> {
  if (!isTauri()) return false;
  try {
    await vivyChat.refreshSessions();
    return true;
  } catch (e) {
    console.error("Failed to fetch sessions:", e);
    return false;
  }
}

/**
 * connect() is the startup restore path: it attaches the event listener,
 * loads session/list + review/list snapshots, and opens the latest session.
 */
/** @returns true when history was applied to the message list */
async function loadSession(sessionKey: string): Promise<boolean> {
  if (!isTauri()) return false;
  syncPlanRuntime(null);
  try {
    return await vivyChat.loadSession(sessionKey);
  } catch (e) {
    console.error("Failed to load session history:", e);
    pushSystemNotice(t('app.errorPrefix') + e);
    return false;
  }
}

async function deleteSession(sessionKey: string) {
  if (!isTauri()) return;
  if (!(await appConfirm(t('chat.confirmDeleteSession')))) return;
  try {
    await vivyChat.deleteSession(sessionKey);
  } catch (e) {
    console.error('Failed to delete session:', e);
    pushSystemNotice(`${t('app.errorPrefix')}${e}`);
  }
}

async function renameSession(sessionKey: string, title: string) {
  if (!isTauri()) return;
  try {
    await vivyChat.renameSession(sessionKey, title);
  } catch (e) {
    console.error('Failed to rename session:', e);
  }
}


async function saveConfig(newConfig: typeof config.value) {
  try {
    if (!isTauri()) {
        console.warn('Running in browser, mocking saveConfig');
        showAppToast("保存成功");
        return;
    }

    // DN-3: VIVY owns provider configuration and credentials. The typed key
    // is written once (write-only — never read back), then provider/model
    // is selected atomically via settings/model/select.
    await saveActiveProvider({
      provider: newConfig.provider,
      model: newConfig.model,
      apiBase: newConfig.apiBase || undefined,
      apiKey: newConfig.apiKey || undefined,
    });

    const snapshot = await loadProviderState();
    providerConfigs.value = snapshot.providerConfigs;
    config.value = {
      provider: newConfig.provider,
      apiBase: newConfig.apiBase,
      apiKey: '',
      model: newConfig.model,
    };

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
    if (dk) {
      await saveConfig(buildWelcomeDeepSeekConfig(config.value, dk));
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

/** Copied controller state -> local refs. The controller is the single
 * orchestration authority for chat/session/review/plan flows (DN-2). */
function syncFromController() {
  messages.value = vivyChat.messages();
  isTyping.value = vivyChat.isTyping();
  const status = vivyChat.status();
  connectionStatus.value =
    status === 'connected' ? 'connected'
    : status === 'error' ? 'error'
    : 'connecting';
  sessions.value = vivyChat.sessions();
  currentSessionKey.value = vivyChat.currentSessionId ?? '';
  unifiedApprovals.value = vivyChat.approvals();
  approvalDetails.value = Object.fromEntries(
    unifiedApprovals.value.map((approval) => [approval.request_id, approval]),
  );
  unifiedOutcomeUnknownIds.value = [...vivyChat.outcomeUnknown];
  unifiedActionErrors.value = Object.fromEntries(vivyChat.actionErrors);
  pendingQuestions.value = vivyChat.questions();
  syncPlanRuntime(vivyChat.plan());
  compactionStatus.value = vivyChat.compaction
    ? { trigger: 'auto', phase: 'completed' }
    : null;
  const pendingCount = unifiedApprovals.value.filter((a) => a.status === 'pending').length;
  if (pendingCount > 0 && !approvalCenterOpen.value && !approvalDrawerAutoOpened.value) {
    // Auto-open the drawer once when a pending review appears.
    approvalCenterOpen.value = true;
    approvalDrawerAutoOpened.value = true;
  }
}

onMounted(async () => {
  try {
    const storedModels = localStorage.getItem(SAVED_MODELS_KEY);
    if (storedModels) savedModels.value = JSON.parse(storedModels);
    const storedPrefs = localStorage.getItem(HISTORY_PREFS_KEY);
    if (storedPrefs) {
      chatDisplayPrefs.value = { ...defaultChatDisplayPrefs, ...JSON.parse(storedPrefs) };
    }
  } catch (_) {
    /* ignore */
  }

  if (!isTauri()) {
      console.log("Running in browser mode - Tauri listeners skipped");
      return;
  }

  try {
    detachVivySync = vivyChat.subscribe(syncFromController);
    await vivyChat.connect().catch((e) => {
      console.error('vivy connect failed:', e);
    });
    syncFromController();
    if (messages.value.length === 0) {
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

    try {
      // DN-3: provider/model state comes from the VIVY settings surface —
      // the active spec name resolves through the status projection so the
      // UI keeps vendor names (deepseek) even though the wire stores bundles.
      const snapshot = await withTimeout(
        loadProviderState(),
        STARTUP_TASK_TIMEOUT_MS,
        "loadProviderState"
      );
      providerConfigs.value = snapshot.providerConfigs;
      const currentSpecName =
        snapshot.statusReport.providers.find((p) => p.current)?.name ||
        snapshot.runtime.provider ||
        config.value.provider;
      const currentSpec = snapshot.providers.find((p) => p.name === currentSpecName);
      config.value = {
        provider: currentSpecName,
        apiBase:
          snapshot.runtime.apiBase ||
          currentSpec?.default_api_base ||
          config.value.apiBase,
        apiKey: '',
        model: snapshot.runtime.model || config.value.model,
      };
      syncCurrentConfigToSavedModels(config.value);
    } catch (e) {
      console.warn("Failed to load provider settings:", e);
    }

    try {
      if (typeof localStorage !== 'undefined' && !localStorage.getItem(WELCOME_STORAGE_KEY)) {
        showWelcomeWizard.value = true;
      }
    } catch (_) {
      /* ignore */
    }
  } catch (e) {
    console.error("App initialization error:", e);
  }
});

onUnmounted(() => {
  detachVivySync?.();
  detachVivySync = null;
});

</script>

<template>
  <div class="w-screen h-screen overflow-hidden bg-transparent relative">
    <WelcomeWizard
      :open="showWelcomeWizard"
      :config="config"
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
      :saved-models="savedModels"
      :sessions="sessions"
      :chat-display-prefs="chatDisplayPrefs"
      :current-session-key="currentSessionKey"
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
      @rename-session="renameSession"
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
