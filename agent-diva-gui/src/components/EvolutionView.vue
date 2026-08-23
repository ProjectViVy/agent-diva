<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { FilePlus2, GitBranch, History, RefreshCw, Save, Search, ShieldCheck, WandSparkles } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import {
  acceptSkillRequest,
  createSkillRequest,
  deleteSkill,
  disableSkill,
  getAutoDreamLiveText,
  getAutoDreamRunStatus,
  getSkill,
  getSkillHistoryRevision,
  getSkillRequest,
  getSkills,
  listAutoDreamRunEvents,
  listAutoDreamRunRecords,
  listSkillHistory,
  listSkillRequests,
  rejectSkillRequest,
  updateSkill,
} from '../api/desktop';
import type {
  AutoDreamRunEvent,
  AutoDreamRunRecord,
  SkillDocument,
  SkillDto,
  SkillHistoryDocument,
  SkillHistoryEntry,
  SkillRequest,
} from '../api/desktop';
import PersonaMarkdownEditor from './persona-memory/PersonaMarkdownEditor.vue';
import { appConfirm } from '../utils/appDialog';
import { errorMessage } from '../utils/errorMessage';
import { showAppToast } from '../utils/appToast';

type EvolutionTab = 'skills' | 'requests' | 'autodream';
type CountTone = 'none' | 'warning';

const props = withDefaults(defineProps<{
  initialTab?: EvolutionTab;
  initialProposalId?: string | null;
  initialSourceRunId?: string | null;
  requestKey?: string | null;
}>(), {
  initialTab: 'requests',
  initialProposalId: null,
  initialSourceRunId: null,
  requestKey: null,
});

const emit = defineEmits<{
  (event: 'count-change', payload: { total: number; tone: CountTone; tooltip: string }): void;
  (event: 'open-settings', view: 'self-evolution'): void;
  (event: 'open-chat'): void;
}>();

const { t } = useI18n();
const activeTab = ref<EvolutionTab>(props.initialTab ?? 'requests');
const skills = ref<SkillDto[]>([]);
const requests = ref<SkillRequest[]>([]);
const skillsLoading = ref(false);
const requestsLoading = ref(false);
const skillsError = ref<string | null>(null);
const requestsError = ref<string | null>(null);
const search = ref('');

const selectedSlug = ref<string | null>(null);
const selectedSkill = ref<SkillDocument | null>(null);
const skillDetailLoading = ref(false);
const skillDetailError = ref<string | null>(null);
let skillDetailToken = 0;

const selectedRequestId = ref<string | null>(props.initialProposalId ?? null);
const selectedRequest = ref<SkillRequest | null>(null);
const requestDetailLoading = ref(false);
const requestDetailError = ref<string | null>(null);
let requestDetailToken = 0;

const runs = ref<AutoDreamRunRecord[]>([]);
const runsLoading = ref(false);
const runsError = ref<string | null>(null);
const selectedRunId = ref<string | null>(props.initialSourceRunId ?? null);
const selectedRun = ref<AutoDreamRunRecord | null>(null);
const runDetailLoading = ref(false);
const runDetailError = ref<string | null>(null);
const runEvents = ref<AutoDreamRunEvent[]>([]);
const runEventsError = ref<string | null>(null);
const runLiveText = ref('');
const runLiveTextError = ref<string | null>(null);
let runDetailToken = 0;
let runPollTimer: ReturnType<typeof setInterval> | null = null;

const editing = ref(false);
const draft = ref('');
const actionError = ref<string | null>(null);
const busyKey = ref<string | null>(null);
const historyOpen = ref(false);
const historyLoading = ref(false);
const historyEntries = ref<SkillHistoryEntry[]>([]);
const historyPreview = ref<SkillHistoryDocument | null>(null);

const createOpen = ref(false);
const createSlug = ref('');
const createTitle = ref('');
const createReason = ref('');
const createAttestation = ref('');
const createMarkdown = ref('---\nname: \ndescription: \nenabled: true\nalways: false\n---\n\n# Skill\n');

const normalizedSearch = computed(() => search.value.trim().toLowerCase());
const filteredSkills = computed(() => skills.value.filter((skill) => {
  const query = normalizedSearch.value;
  return !query || skill.slug.includes(query) || skill.description.toLowerCase().includes(query);
}));
const filteredRequests = computed(() => requests.value.filter((request) => {
  const query = normalizedSearch.value;
  return !query || request.slug.includes(query) || request.title.toLowerCase().includes(query);
}));
const filteredRuns = computed(() => runs.value.filter((run) => {
  const query = normalizedSearch.value;
  if (!query) return true;
  return [
    run.id,
    run.trigger,
    run.state,
    run.orchestration?.phase ?? '',
    run.summary ?? '',
    run.error ?? '',
  ].some((value) => value.toLowerCase().includes(query));
}));
const pendingCount = computed(() => requests.value.filter((request) => request.status === 'pending').length);
const createBaseHash = computed(() => skills.value.find((skill) => skill.slug === createSlug.value.trim())?.content_hash ?? '0');

function normalizeError(error: unknown) {
  return errorMessage(error, 'Evolution 请求失败');
}

function isActiveRun(run: AutoDreamRunRecord | null | undefined) {
  return run?.state === 'pending' || run?.state === 'running';
}

function formatRunTimestamp(value?: string | null) {
  if (!value) return '—';
  const timestamp = new Date(value);
  return Number.isNaN(timestamp.getTime()) ? value : timestamp.toLocaleString();
}

function displayRunLiveText(value: string) {
  return value
    .replace(/"excerpt"\s*:\s*"(?:\\.|[^"])*"/g, '"excerpt":"[redacted]"')
    .replace(/"hash"\s*:\s*"(?:\\.|[^"])*"/g, '"hash":"[redacted]"')
    .replace(/"uri"\s*:\s*"(?:\\.|[^"])*"/g, '"uri":"[redacted]"')
    .replace(/"workspace_id"\s*:\s*"(?:\\.|[^"])*"/g, '"workspace_id":"[redacted]"');
}

function runInputSummary(run: AutoDreamRunRecord) {
  const input = run.input_summary;
  if (!input) return '—';
  return `${input.total_items} 项 · ${input.total_bytes} bytes${input.truncated ? ' · truncated' : ''}`;
}

function runPhaseLabel(run: AutoDreamRunRecord) {
  return run.orchestration?.phase
    ? t(`evolution.autodream.phases.${run.orchestration.phase}`)
    : t(`evolution.autodream.states.${run.state}`);
}

function stopRunPolling() {
  if (runPollTimer) clearInterval(runPollTimer);
  runPollTimer = null;
}

function syncRunPolling() {
  stopRunPolling();
  if (activeTab.value !== 'autodream' || !isActiveRun(selectedRun.value)) return;
  const runId = selectedRun.value?.id;
  if (!runId) return;
  runPollTimer = setInterval(() => {
    void refreshRunDetails(runId, false);
  }, 1000);
}

async function refreshRunDetails(id = selectedRunId.value, showLoading = true) {
  if (!id) return;
  const token = ++runDetailToken;
  if (showLoading) runDetailLoading.value = true;
  runDetailError.value = null;
  try {
    const nextRun = await getAutoDreamRunStatus(id);
    if (token !== runDetailToken || selectedRunId.value !== id) return;
    selectedRun.value = nextRun;
    runs.value = runs.value.map((run) => run.id === nextRun.id ? nextRun : run);

    const [eventsResult, liveTextResult] = await Promise.allSettled([
      listAutoDreamRunEvents(id),
      getAutoDreamLiveText(id),
    ]);
    if (token !== runDetailToken || selectedRunId.value !== id) return;

    if (eventsResult.status === 'fulfilled') {
      runEvents.value = eventsResult.value;
      runEventsError.value = null;
    } else {
      runEventsError.value = normalizeError(eventsResult.reason);
    }
    if (liveTextResult.status === 'fulfilled') {
      runLiveText.value = liveTextResult.value;
      runLiveTextError.value = null;
    } else {
      runLiveTextError.value = normalizeError(liveTextResult.reason);
    }
    syncRunPolling();
  } catch (error) {
    if (token === runDetailToken && selectedRunId.value === id) {
      runDetailError.value = normalizeError(error);
    }
  } finally {
    if (token === runDetailToken) runDetailLoading.value = false;
  }
}

async function selectRun(id: string) {
  selectedRunId.value = id;
  selectedRun.value = runs.value.find((run) => run.id === id) ?? null;
  runEvents.value = [];
  runLiveText.value = '';
  runDetailError.value = null;
  runEventsError.value = null;
  runLiveTextError.value = null;
  stopRunPolling();
  await refreshRunDetails(id);
}

async function loadRuns() {
  runsLoading.value = true;
  runsError.value = null;
  try {
    const next = await listAutoDreamRunRecords();
    runs.value = next;
    if (selectedRunId.value) {
      const selected = next.find((run) => run.id === selectedRunId.value);
      if (selected) {
        selectedRun.value = selected;
      } else {
        selectedRunId.value = null;
        selectedRun.value = null;
        stopRunPolling();
      }
    }
  } catch (error) {
    runsError.value = normalizeError(error);
  } finally {
    runsLoading.value = false;
  }
}

function emitCount() {
  emit('count-change', {
    total: pendingCount.value,
    tone: pendingCount.value > 0 ? 'warning' : 'none',
    tooltip: pendingCount.value > 0 ? `${pendingCount.value} 条 Skill 待审请求` : '没有 Skill 待审请求',
  });
}

async function loadSkills() {
  skillsLoading.value = true;
  skillsError.value = null;
  try {
    const next = (await getSkills()).filter((skill) => skill.evolution_managed === true);
    skills.value = next;
    if (selectedSlug.value && !next.some((skill) => skill.slug === selectedSlug.value)) {
      selectedSlug.value = null;
      selectedSkill.value = null;
    }
  } catch (error) {
    skillsError.value = normalizeError(error);
  } finally {
    skillsLoading.value = false;
  }
}

async function loadRequests() {
  requestsLoading.value = true;
  requestsError.value = null;
  try {
    const next = await listSkillRequests();
    requests.value = next;
    emitCount();
    if (selectedRequestId.value && !next.some((request) => request.id === selectedRequestId.value)) {
      selectedRequestId.value = null;
      selectedRequest.value = null;
    }
  } catch (error) {
    requestsError.value = normalizeError(error);
  } finally {
    requestsLoading.value = false;
  }
}

async function refresh() {
  await Promise.all([loadSkills(), loadRequests(), loadRuns()]);
  await ensureRunSelection();
}

async function selectSkill(slug: string) {
  selectedSlug.value = slug;
  selectedSkill.value = null;
  editing.value = false;
  historyOpen.value = false;
  historyPreview.value = null;
  skillDetailError.value = null;
  const token = ++skillDetailToken;
  skillDetailLoading.value = true;
  try {
    const detail = await getSkill(slug);
    if (token !== skillDetailToken || selectedSlug.value !== slug) return;
    selectedSkill.value = detail;
    draft.value = detail.markdown;
  } catch (error) {
    if (token === skillDetailToken && selectedSlug.value === slug) {
      skillDetailError.value = normalizeError(error);
    }
  } finally {
    if (token === skillDetailToken) skillDetailLoading.value = false;
  }
}

async function selectRequest(id: string) {
  selectedRequestId.value = id;
  selectedRequest.value = null;
  requestDetailError.value = null;
  const token = ++requestDetailToken;
  requestDetailLoading.value = true;
  try {
    const detail = await getSkillRequest(id);
    if (token !== requestDetailToken || selectedRequestId.value !== id) return;
    selectedRequest.value = detail;
  } catch (error) {
    if (token === requestDetailToken && selectedRequestId.value === id) {
      requestDetailError.value = normalizeError(error);
    }
  } finally {
    if (token === requestDetailToken) requestDetailLoading.value = false;
  }
}

async function ensureRunSelection() {
  if (activeTab.value !== 'autodream') return;
  const nextId = selectedRunId.value && runs.value.some((run) => run.id === selectedRunId.value)
    ? selectedRunId.value
    : runs.value[0]?.id ?? null;
  if (!nextId) {
    selectedRunId.value = null;
    selectedRun.value = null;
    stopRunPolling();
    return;
  }
  if (selectedRunId.value !== nextId || selectedRun.value?.id !== nextId) {
    await selectRun(nextId);
  } else {
    await refreshRunDetails(nextId);
  }
}

async function openRunRequests() {
  const proposalId = selectedRun.value?.proposal_ids?.[0];
  if (!proposalId) return;
  activeTab.value = 'requests';
  await loadRequests();
  if (requests.value.some((request) => request.id === proposalId)) {
    await selectRequest(proposalId);
  }
}

async function withAction(key: string, action: () => Promise<void>) {
  busyKey.value = key;
  actionError.value = null;
  try {
    await action();
  } catch (error) {
    actionError.value = normalizeError(error);
    showAppToast(actionError.value, 'error');
  } finally {
    busyKey.value = null;
  }
}

function beginEdit() {
  if (!selectedSkill.value) return;
  draft.value = selectedSkill.value.markdown;
  editing.value = true;
  actionError.value = null;
}

async function saveSkill() {
  const skill = selectedSkill.value;
  if (!skill) return;
  await withAction(`save:${skill.slug}`, async () => {
    const outcome = await updateSkill(skill.slug, draft.value, skill.content_hash);
    selectedSkill.value = outcome.document;
    draft.value = outcome.document.markdown;
    editing.value = false;
    await loadSkills();
    showAppToast(outcome.changed ? 'Skill 已保存' : '内容未变化', 'success');
  });
}

async function disableSelectedSkill() {
  const skill = selectedSkill.value;
  if (!skill) return;
  await withAction(`disable:${skill.slug}`, async () => {
    const outcome = await disableSkill(skill.slug, skill.content_hash);
    selectedSkill.value = outcome.document;
    draft.value = outcome.document.markdown;
    await loadSkills();
    showAppToast('Skill 已停用', 'success');
  });
}

async function deleteSelectedSkill() {
  const skill = selectedSkill.value;
  if (!skill?.can_hard_delete) return;
  const confirmed = await appConfirm(`确认硬删除 Home Skill “${skill.slug}”？同名内置 Skill 将重新显示。`, {
    title: '硬删除 Skill',
  });
  if (!confirmed) return;
  await withAction(`delete:${skill.slug}`, async () => {
    await deleteSkill(skill.slug, skill.content_hash);
    selectedSlug.value = null;
    selectedSkill.value = null;
    await loadSkills();
    showAppToast('Home Skill 已删除', 'success');
  });
}

async function toggleHistory() {
  const skill = selectedSkill.value;
  if (!skill) return;
  historyOpen.value = !historyOpen.value;
  if (!historyOpen.value) return;
  historyLoading.value = true;
  try {
    historyEntries.value = await listSkillHistory(skill.slug);
  } catch (error) {
    actionError.value = normalizeError(error);
  } finally {
    historyLoading.value = false;
  }
}

async function previewHistory(revision: number) {
  const slug = selectedSkill.value?.slug;
  if (!slug) return;
  const expectedSlug = slug;
  try {
    const next = await getSkillHistoryRevision(slug, revision);
    if (selectedSkill.value?.slug === expectedSlug) historyPreview.value = next;
  } catch (error) {
    actionError.value = normalizeError(error);
  }
}

async function decideRequest(action: 'accept' | 'reject') {
  const request = selectedRequest.value;
  if (!request || request.status !== 'pending') return;
  if (action === 'reject') {
    const confirmed = await appConfirm(`拒绝 “${request.title}”？`, { title: '拒绝 Skill 请求' });
    if (!confirmed) return;
  }
  await withAction(`${action}:${request.id}`, async () => {
    const next = action === 'accept'
      ? await acceptSkillRequest(request.id)
      : await rejectSkillRequest(request.id);
    selectedRequest.value = next;
    requests.value = requests.value.map((item) => item.id === next.id ? next : item);
    emitCount();
    if (action === 'accept') await loadSkills();
    showAppToast(action === 'accept' ? '请求已接受' : '请求已拒绝', 'success');
  });
}

async function submitRequest() {
  const slug = createSlug.value.trim();
  if (!slug || !createTitle.value.trim() || !createReason.value.trim()) {
    actionError.value = 'slug、标题和原因不能为空';
    return;
  }
  await withAction('create-request', async () => {
    const request = await createSkillRequest({
      slug,
      title: createTitle.value.trim(),
      proposed_markdown: createMarkdown.value,
      evidence: [],
      attestation: createAttestation.value.trim(),
      base_hash: createBaseHash.value,
      reason: createReason.value.trim(),
    });
    requests.value = [request, ...requests.value.filter((item) => item.id !== request.id)];
    createOpen.value = false;
    activeTab.value = 'requests';
    emitCount();
    await selectRequest(request.id);
    showAppToast('Skill 待审请求已创建', 'success');
  });
}

function closeMobileDetail() {
  if (activeTab.value === 'skills') {
    selectedSlug.value = null;
    selectedSkill.value = null;
  } else if (activeTab.value === 'requests') {
    selectedRequestId.value = null;
    selectedRequest.value = null;
  } else {
    selectedRunId.value = null;
    selectedRun.value = null;
    stopRunPolling();
  }
}

watch(() => props.requestKey, async () => {
  activeTab.value = props.initialTab ?? 'requests';
  if (props.initialProposalId) {
    await loadRequests();
    if (requests.value.some((item) => item.id === props.initialProposalId)) {
      await selectRequest(props.initialProposalId);
    }
  }
  if (activeTab.value === 'autodream') {
    selectedRunId.value = props.initialSourceRunId ?? selectedRunId.value;
    await loadRuns();
    await ensureRunSelection();
  }
});

watch(activeTab, async (tab, previousTab) => {
  if (previousTab === 'autodream' && tab !== 'autodream') stopRunPolling();
  if (tab === 'autodream') {
    await loadRuns();
    await ensureRunSelection();
  }
});

watch([activeTab, () => selectedRun.value?.state], () => {
  syncRunPolling();
});

watch(createSlug, (slug) => {
  createMarkdown.value = createMarkdown.value.replace(/^name:\s*.*$/m, `name: ${slug.trim()}`);
});

onMounted(async () => {
  await refresh();
  if (props.initialProposalId && requests.value.some((item) => item.id === props.initialProposalId)) {
    await selectRequest(props.initialProposalId);
  }
});

onBeforeUnmount(() => {
  stopRunPolling();
});
</script>

<template>
  <section class="evolution-shell">
    <header class="evolution-header">
      <div>
        <p class="eyebrow">EVOLUTION</p>
        <h1>Skill 进化</h1>
        <p>机器级 Skill 权威与可审计的待审请求。</p>
      </div>
      <div class="header-actions">
        <button class="secondary" type="button" @click="emit('open-settings', 'self-evolution')">AutoDream 设置</button>
        <button class="icon-button" type="button" aria-label="刷新" @click="refresh"><RefreshCw :size="17" /></button>
      </div>
    </header>

    <nav class="tabbar" aria-label="Evolution sections">
      <button :class="{ active: activeTab === 'skills' }" @click="activeTab = 'skills'">
        <WandSparkles :size="17" /> Skill <span>{{ skills.length }}</span>
      </button>
      <button :class="{ active: activeTab === 'requests' }" @click="activeTab = 'requests'">
        <ShieldCheck :size="17" /> 待审 <span>{{ pendingCount }}</span>
      </button>
      <button :class="{ active: activeTab === 'autodream' }" @click="activeTab = 'autodream'">
        <GitBranch :size="17" /> {{ t('evolution.autodream.tab') }} <span>{{ runs.length }}</span>
      </button>
    </nav>

    <div class="toolbar">
      <label class="search-box"><Search :size="16" /><input v-model="search" :placeholder="activeTab === 'autodream' ? t('evolution.autodream.search') : '搜索 slug、描述或标题'" /></label>
      <button v-if="activeTab === 'requests'" class="primary" type="button" @click="createOpen = !createOpen">
        <FilePlus2 :size="16" /> 新建请求
      </button>
    </div>

    <div v-if="createOpen" class="create-panel">
      <div class="form-grid">
        <label>Slug<input v-model="createSlug" placeholder="my-skill" /></label>
        <label>标题<input v-model="createTitle" placeholder="可审阅的变更标题" /></label>
        <label class="wide">原因<input v-model="createReason" placeholder="为什么需要这个 Skill" /></label>
        <label class="wide">用户声明<input v-model="createAttestation" placeholder="没有 evidence 时必须填写非空声明" /></label>
      </div>
      <div class="editor-frame create-editor"><PersonaMarkdownEditor v-model="createMarkdown" /></div>
      <div class="panel-actions"><button class="secondary" @click="createOpen = false">取消</button><button class="primary" :disabled="busyKey === 'create-request'" @click="submitRequest">提交待审</button></div>
    </div>

    <p v-if="actionError" class="error-banner">{{ actionError }}</p>

    <div v-if="activeTab === 'autodream'" class="workspace autodream-workspace" :class="{ 'has-detail': selectedRunId }">
      <aside class="master-list">
        <p v-if="runsError" class="state error">{{ t('evolution.autodream.errorTitle') }}：{{ runsError }} <button @click="refresh">{{ t('evolution.autodream.retry') }}</button></p>
        <p v-if="runsLoading && runs.length > 0" class="autodream-refreshing" role="status">{{ t('evolution.autodream.refreshing') }}</p>
        <p v-else-if="runsLoading && runs.length === 0" class="state">{{ t('evolution.autodream.loading') }}</p>
        <template v-if="!runsError && runs.length === 0 && !runsLoading">
          <p class="state">{{ t('evolution.autodream.emptyTitle') }}<br /><small>{{ t('evolution.autodream.emptyDesc') }}</small><br /><button @click="emit('open-chat')">{{ t('evolution.autodream.openChat') }}</button></p>
        </template>
        <template v-if="runs.length > 0">
          <p v-if="filteredRuns.length === 0" class="state">{{ t('evolution.autodream.filteredEmpty') }}</p>
          <button v-for="run in filteredRuns" :key="run.id" class="list-row" :class="{ selected: selectedRunId === run.id }" @click="selectRun(run.id)">
            <span class="row-title">{{ run.id }}</span><span class="status" :class="run.state">{{ t(`evolution.autodream.states.${run.state}`) }}</span>
          <span class="row-description">{{ run.trigger }} · {{ runPhaseLabel(run) }}</span>
            <span class="row-meta">{{ formatRunTimestamp(run.started_at) }}</span>
          </button>
        </template>
      </aside>

      <main class="detail-pane autodream-detail-pane">
        <button class="mobile-back" type="button" @click="closeMobileDetail">← 返回列表</button>
        <p v-if="!selectedRunId" class="detail-empty">{{ t('evolution.autodream.select') }}</p>
        <p v-else-if="runDetailLoading && !selectedRun" class="state">{{ t('evolution.autodream.loading') }}</p>
        <p v-else-if="runDetailError && !selectedRun" class="state error">{{ runDetailError }} <button @click="refreshRunDetails()">{{ t('evolution.autodream.retry') }}</button></p>
        <template v-else-if="selectedRun">
          <div class="detail-heading">
            <div><p class="eyebrow">{{ t('evolution.autodream.title') }}</p><h2>{{ selectedRun.id }}</h2><p>{{ selectedRun.summary || 'AutoDream' }}</p></div>
            <span class="status" :class="selectedRun.state">{{ t(`evolution.autodream.states.${selectedRun.state}`) }}</span>
          </div>
          <p v-if="runDetailError" class="error-banner">{{ runDetailError }} <button @click="refreshRunDetails()">{{ t('evolution.autodream.retry') }}</button></p>
          <div class="facts autodream-facts">
            <span>{{ t('evolution.autodream.trigger') }}: {{ selectedRun.trigger }}</span>
            <span>{{ t('evolution.autodream.phase') }}: {{ runPhaseLabel(selectedRun) }}</span>
            <span>{{ t('evolution.autodream.startedAt') }}: {{ formatRunTimestamp(selectedRun.started_at) }}</span>
            <span>{{ t('evolution.autodream.completedAt') }}: {{ formatRunTimestamp(selectedRun.completed_at) }}</span>
            <span>{{ t('evolution.autodream.attempt') }}: {{ selectedRun.orchestration?.attempt ?? 0 }}</span>
            <span>{{ t('evolution.autodream.updatedAt') }}: {{ formatRunTimestamp(selectedRun.orchestration?.updated_at) }}</span>
          </div>
          <dl class="autodream-record-grid">
            <div><dt>{{ t('evolution.autodream.inputs') }}</dt><dd>{{ runInputSummary(selectedRun) }}</dd></div>
            <div><dt>{{ t('evolution.autodream.proposals') }}</dt><dd>{{ selectedRun.proposal_ids.length }}</dd></div>
            <div v-if="selectedRun.orchestration"><dt>{{ t('evolution.autodream.deadline') }}</dt><dd>{{ formatRunTimestamp(selectedRun.orchestration.deadline_at) }}</dd></div>
            <div v-if="selectedRun.error"><dt>{{ t('evolution.autodream.failure') }}</dt><dd class="autodream-error-text">{{ selectedRun.error }}</dd></div>
          </dl>
          <button v-if="selectedRun.proposal_ids.length > 0" class="secondary autodream-open-requests" type="button" @click="openRunRequests">{{ t('evolution.autodream.openRequests') }}</button>
          <section class="autodream-output-panel">
            <h3>{{ t('evolution.autodream.liveOutput') }}</h3>
            <p v-if="runLiveTextError" class="state error">{{ t('evolution.autodream.liveOutputError') }}：{{ runLiveTextError }}</p>
            <pre v-else-if="runLiveText">{{ displayRunLiveText(runLiveText) }}</pre>
            <p v-else class="state">{{ t('evolution.autodream.liveOutputEmpty') }}</p>
          </section>
          <section class="autodream-events-panel">
            <h3>{{ t('evolution.autodream.events') }}</h3>
            <p v-if="runEventsError" class="state error">{{ t('evolution.autodream.eventsError') }}：{{ runEventsError }}</p>
            <ol v-else-if="runEvents.length > 0">
              <li v-for="event in runEvents" :key="event.id"><time>{{ formatRunTimestamp(event.created_at) }}</time><strong>{{ event.kind }}</strong><span>{{ event.message }}</span></li>
            </ol>
            <p v-else class="state">{{ t('evolution.autodream.eventsEmpty') }}</p>
          </section>
        </template>
      </main>
    </div>

    <div v-else class="workspace" :class="{ 'has-detail': selectedSlug || selectedRequestId }">
      <aside class="master-list">
        <template v-if="activeTab === 'skills'">
          <p v-if="skillsError" class="state error">加载失败：{{ skillsError }} <button @click="loadSkills">重试</button></p>
          <p v-else-if="skillsLoading && skills.length === 0" class="state">正在加载 Skill…</p>
          <p v-else-if="skills.length === 0" class="state">尚无 Evolution 管理的 Skill。</p>
          <p v-else-if="filteredSkills.length === 0" class="state">没有符合筛选条件的 Skill。</p>
          <button v-for="skill in filteredSkills" :key="skill.slug" class="list-row" :class="{ selected: selectedSlug === skill.slug }" @click="selectSkill(skill.slug)">
            <span class="row-title">{{ skill.slug }}</span><span class="source" :class="skill.source">{{ skill.source }}</span>
            <span class="row-description">{{ skill.description }}</span>
            <span class="row-meta"><span :class="{ muted: !skill.enabled }">{{ skill.enabled ? '启用' : '停用' }}</span><span v-if="skill.always">always</span></span>
          </button>
        </template>
        <template v-else>
          <p v-if="requestsError" class="state error">加载失败：{{ requestsError }} <button @click="loadRequests">重试</button></p>
          <p v-else-if="requestsLoading && requests.length === 0" class="state">正在加载待审请求…</p>
          <p v-else-if="requests.length === 0" class="state">没有 Skill 请求。</p>
          <p v-else-if="filteredRequests.length === 0" class="state">没有符合筛选条件的请求。</p>
          <button v-for="request in filteredRequests" :key="request.id" class="list-row" :class="{ selected: selectedRequestId === request.id }" @click="selectRequest(request.id)">
            <span class="row-title">{{ request.title }}</span><span class="status" :class="request.status">{{ request.status }}</span>
            <span class="row-description">{{ request.slug }} · {{ request.source }}</span>
            <span class="row-meta">{{ new Date(request.updated_at).toLocaleString() }}</span>
          </button>
        </template>
      </aside>

      <main class="detail-pane">
        <button class="mobile-back" type="button" @click="closeMobileDetail">← 返回列表</button>
        <template v-if="activeTab === 'skills'">
          <p v-if="!selectedSlug" class="detail-empty">选择一个 Skill 查看详情。</p>
          <p v-else-if="skillDetailLoading" class="state">正在加载详情…</p>
          <p v-else-if="skillDetailError" class="state error">{{ skillDetailError }} <button @click="selectSkill(selectedSlug)">重试</button></p>
          <template v-else-if="selectedSkill">
            <div class="detail-heading">
              <div><p class="eyebrow">{{ selectedSkill.source }}</p><h2>{{ selectedSkill.slug }}</h2><p>{{ selectedSkill.description }}</p></div>
              <div class="detail-actions">
                <button v-if="!editing" class="secondary" @click="beginEdit">编辑</button>
                <template v-else><button class="secondary" @click="editing = false; draft = selectedSkill.markdown">取消</button><button class="primary" :disabled="busyKey === `save:${selectedSkill.slug}`" @click="saveSkill"><Save :size="15" /> 保存</button></template>
                <button class="secondary" :disabled="!selectedSkill.enabled || Boolean(busyKey)" @click="disableSelectedSkill">停用</button>
                <button class="danger" :disabled="!selectedSkill.can_hard_delete || Boolean(busyKey)" @click="deleteSelectedSkill">硬删</button>
              </div>
            </div>
            <div class="facts"><span>hash {{ selectedSkill.content_hash.slice(0, 12) }}</span><span>{{ selectedSkill.always ? 'always' : 'on demand' }}</span><span>{{ selectedSkill.enabled ? 'enabled' : 'disabled' }}</span></div>
            <div class="editor-frame"><PersonaMarkdownEditor :key="editing ? 'edit' : 'read'" v-model="draft" :readonly="!editing" /></div>
            <button class="history-toggle" @click="toggleHistory"><History :size="16" /> 历史快照</button>
            <div v-if="historyOpen" class="history-panel">
              <p v-if="historyLoading">正在加载历史…</p><p v-else-if="historyEntries.length === 0">暂无 Home 历史。</p>
              <button v-for="entry in historyEntries" :key="entry.revision" @click="previewHistory(entry.revision)">#{{ entry.revision }} · {{ entry.content_hash.slice(0, 12) }} · {{ new Date(entry.updated_at).toLocaleString() }}</button>
              <pre v-if="historyPreview">{{ historyPreview.markdown }}</pre>
            </div>
          </template>
        </template>

        <template v-else>
          <p v-if="!selectedRequestId" class="detail-empty">选择一条请求进行审阅。</p>
          <p v-else-if="requestDetailLoading" class="state">正在加载请求…</p>
          <p v-else-if="requestDetailError" class="state error">{{ requestDetailError }} <button @click="selectRequest(selectedRequestId)">重试</button></p>
          <template v-else-if="selectedRequest">
            <div class="detail-heading">
              <div><p class="eyebrow">{{ selectedRequest.source }}</p><h2>{{ selectedRequest.title }}</h2><p>{{ selectedRequest.slug }} · {{ selectedRequest.reason }}</p></div>
              <div class="detail-actions"><button class="secondary" :disabled="selectedRequest.status !== 'pending' || Boolean(busyKey)" @click="decideRequest('reject')">拒绝</button><button class="primary" :disabled="selectedRequest.status !== 'pending' || Boolean(busyKey)" @click="decideRequest('accept')">接受</button></div>
            </div>
            <div class="facts"><span class="status" :class="selectedRequest.status">{{ selectedRequest.status }}</span><span>base {{ selectedRequest.base_hash.slice(0, 12) }}</span><span>{{ new Date(selectedRequest.created_at).toLocaleString() }}</span></div>
            <div v-if="selectedRequest.status === 'stale'" class="stale-notice">请求已 stale，不能接受；请依据当前 Skill 头重新创建。</div>
            <div class="editor-frame"><PersonaMarkdownEditor :key="selectedRequest.id" :model-value="selectedRequest.proposed_markdown" readonly /></div>
            <section class="evidence-panel"><h3>Evidence / Attestation</h3><p v-if="selectedRequest.attestation"><strong>声明：</strong>{{ selectedRequest.attestation }}</p><p v-if="selectedRequest.evidence.length === 0">没有 evidence 项。</p><pre v-for="(item, index) in selectedRequest.evidence" :key="index">{{ JSON.stringify(item, null, 2) }}</pre></section>
          </template>
        </template>
      </main>
    </div>
  </section>
</template>

<style scoped>
.evolution-shell { height: 100%; min-height: 0; display: flex; flex-direction: column; padding: var(--space-5, 24px); gap: 14px; color: var(--text); background: radial-gradient(circle at 85% 0%, color-mix(in srgb, var(--accent) 11%, transparent), transparent 34%); }
.evolution-header, .toolbar, .detail-heading, .header-actions, .detail-actions, .panel-actions, .facts, .row-meta { display: flex; align-items: center; }
.evolution-header, .toolbar, .detail-heading { justify-content: space-between; gap: var(--space-4, 16px); }
h1, h2, h3, p { margin: 0; } h1 { font-size: 25px; } h2 { font-size: 21px; } .evolution-header p, .detail-heading p { color: var(--text-muted); margin-top: 5px; }
.eyebrow { color: var(--accent) !important; font: 700 11px/1 var(--font-mono, monospace); letter-spacing: .15em; text-transform: uppercase; }
button, input { font: inherit; } button { cursor: pointer; } button:disabled { cursor: not-allowed; opacity: .48; }
.icon-button, .secondary, .primary, .danger, .history-toggle { display: inline-flex; align-items: center; justify-content: center; gap: 7px; border-radius: 9px; min-height: 36px; padding: 0 13px; border: 1px solid var(--border); background: var(--panel-solid); color: var(--text); }
.icon-button { width: 38px; padding: 0; } .primary { color: white; background: var(--accent); border-color: var(--accent); } .danger { color: var(--danger); border-color: color-mix(in srgb, var(--danger) 45%, var(--border)); }
.header-actions, .detail-actions, .panel-actions, .facts, .row-meta { gap: var(--space-2, 8px); flex-wrap: wrap; }
.tabbar { display: flex; gap: 6px; padding: 5px; width: fit-content; border: 1px solid var(--border); border-radius: 12px; background: var(--panel-solid); }
.tabbar button { display: flex; align-items: center; gap: var(--space-2, 8px); padding: var(--space-2, 8px) 14px; border: 0; border-radius: 8px; color: var(--text-muted); background: transparent; }
.tabbar button.active { color: var(--text); background: color-mix(in srgb, var(--accent) 13%, var(--panel-solid)); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 28%, transparent); }
.tabbar span { min-width: 20px; padding: 1px 6px; border-radius: 999px; background: color-mix(in srgb, var(--text) 8%, transparent); font-size: 11px; }
.search-box { flex: 1; max-width: 480px; display: flex; align-items: center; gap: var(--space-2, 8px); min-height: 39px; padding: 0 var(--space-3, 12px); border: 1px solid var(--border); border-radius: 10px; background: var(--panel-solid); color: var(--text-muted); }
.search-box input, .form-grid input { width: 100%; border: 0; outline: 0; color: var(--text); background: transparent; }
.workspace { min-height: 0; flex: 1; display: grid; grid-template-columns: minmax(250px, 330px) minmax(0, 1fr); overflow: hidden; border: 1px solid var(--border); border-radius: 14px; background: color-mix(in srgb, var(--panel-solid) 92%, transparent); }
.master-list { min-height: 0; overflow: auto; padding: 9px; border-right: 1px solid var(--border); }
.list-row { width: 100%; display: grid; grid-template-columns: 1fr auto; gap: 5px 9px; padding: var(--space-3, 12px); border: 1px solid transparent; border-radius: 10px; background: transparent; color: var(--text); text-align: left; }
.list-row:hover, .list-row.selected { background: color-mix(in srgb, var(--accent) 8%, transparent); border-color: color-mix(in srgb, var(--accent) 24%, transparent); }
.row-title { min-width: 0; overflow: hidden; font-weight: 650; text-overflow: ellipsis; white-space: nowrap; }.row-description { grid-column: 1 / -1; overflow: hidden; color: var(--text-muted); font-size: var(--font-size-xs, 12px); text-overflow: ellipsis; white-space: nowrap; }.row-meta { grid-column: 1 / -1; color: var(--text-muted); font-size: 11px; }
.source, .status { width: fit-content; padding: 2px 7px; border-radius: 999px; font: 700 10px/1.5 var(--font-mono, monospace); text-transform: uppercase; background: color-mix(in srgb, var(--accent) 12%, transparent); color: var(--accent); }.source.builtin { color: var(--text-muted); background: color-mix(in srgb, var(--text) 7%, transparent); }.status.stale, .status.rejected { color: var(--danger); background: color-mix(in srgb, var(--danger) 10%, transparent); }.status.accepted { color: var(--success); background: color-mix(in srgb, var(--success) 10%, transparent); }
.status.running, .status.pending { color: var(--accent); background: color-mix(in srgb, var(--accent) 10%, transparent); }.status.completed { color: var(--success); background: color-mix(in srgb, var(--success) 10%, transparent); }.status.failed, .status.cancelled { color: var(--danger); background: color-mix(in srgb, var(--danger) 10%, transparent); }
.detail-pane { min-height: 0; overflow: auto; padding: 20px; }.detail-empty, .state { padding: var(--space-6, 32px) var(--space-4, 16px); color: var(--text-muted); text-align: center; }.state.error, .error-banner { color: var(--danger); }.state button { border: 0; color: var(--accent); background: none; }
.facts { margin: 13px 0; color: var(--text-muted); font: 11px var(--font-mono, monospace); }.facts > span { padding: var(--space-1, 4px) var(--space-2, 8px); border-radius: 7px; background: color-mix(in srgb, var(--text) 6%, transparent); }.muted { color: var(--text-muted); text-decoration: line-through; }
.autodream-detail-pane { display: flex; flex-direction: column; gap: var(--space-3, 12px); }.autodream-facts { margin: 0; }.autodream-record-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 10px; margin: 0; }.autodream-record-grid > div { min-width: 0; padding: 10px var(--space-3, 12px); border: 1px solid var(--border); border-radius: 9px; background: color-mix(in srgb, var(--text) 4%, transparent); }.autodream-record-grid dt { color: var(--text-muted); font-size: 11px; }.autodream-record-grid dd { margin: var(--space-1, 4px) 0 0; overflow-wrap: anywhere; color: var(--text); font-size: var(--font-size-xs, 12px); line-height: 1.45; }.autodream-error-text { color: var(--danger) !important; }.autodream-open-requests { align-self: flex-start; }.autodream-output-panel, .autodream-events-panel { display: grid; gap: var(--space-2, 8px); padding: var(--space-3, 12px); border: 1px solid var(--border); border-radius: 11px; background: var(--panel-solid); }.autodream-output-panel h3, .autodream-events-panel h3 { font-size: var(--font-size-base, 14px); }.autodream-output-panel pre { max-height: 240px; overflow: auto; margin: 0; padding: 10px; border-radius: 8px; background: color-mix(in srgb, var(--text) 5%, transparent); white-space: pre-wrap; word-break: break-word; }.autodream-events-panel ol { display: grid; gap: var(--space-2, 8px); max-height: 280px; overflow: auto; margin: 0; padding: 0; list-style: none; }.autodream-events-panel li { display: grid; grid-template-columns: minmax(130px, auto) minmax(120px, auto) 1fr; gap: 9px; padding: 9px; border-radius: 8px; background: color-mix(in srgb, var(--text) 5%, transparent); font-size: var(--font-size-xs, 12px); }.autodream-events-panel time { color: var(--text-muted); }.autodream-events-panel strong { color: var(--accent); }.autodream-events-panel span { overflow-wrap: anywhere; }
.autodream-refreshing { margin: var(--space-1, 4px) var(--space-2, 8px) var(--space-2, 8px); color: var(--text-muted); font-size: 11px; }
.editor-frame { height: min(54vh, 560px); min-height: 260px; overflow: hidden; border: 1px solid var(--border); border-radius: 11px; background: color-mix(in srgb, var(--panel-solid) 88%, transparent); }.history-toggle { margin-top: 12px; }.history-panel, .evidence-panel, .create-panel { padding: 14px; border: 1px solid var(--border); border-radius: 11px; background: var(--panel-solid); }.history-panel { margin-top: 8px; }.history-panel button { display: block; width: 100%; padding: var(--space-2, 8px); border: 0; color: var(--text-muted); background: transparent; text-align: left; }.history-panel pre, .evidence-panel pre { overflow: auto; padding: 10px; border-radius: 8px; background: color-mix(in srgb, var(--text) 5%, transparent); white-space: pre-wrap; }.evidence-panel { margin-top: 12px; display: grid; gap: 9px; }.stale-notice, .error-banner { padding: 10px var(--space-3, 12px); border: 1px solid color-mix(in srgb, var(--danger) 35%, transparent); border-radius: 9px; background: color-mix(in srgb, var(--danger) 7%, transparent); }
.create-panel { display: grid; gap: var(--space-3, 12px); }.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }.form-grid label { display: grid; gap: 5px; color: var(--text-muted); font-size: var(--font-size-xs, 12px); }.form-grid label.wide { grid-column: 1 / -1; }.form-grid input { min-height: 36px; padding: 0 10px; border: 1px solid var(--border); border-radius: 8px; }.create-editor { height: 260px; }.panel-actions { justify-content: flex-end; }.mobile-back { display: none; }
@media (max-width: 760px) { .evolution-shell { padding: 14px; }.evolution-header { align-items: flex-start; }.evolution-header > div:first-child p:last-child { display: none; }.workspace { display: block; }.detail-pane { display: none; height: 100%; }.workspace.has-detail .master-list { display: none; }.workspace.has-detail .detail-pane { display: block; }.mobile-back { display: inline-flex; margin-bottom: 12px; border: 0; color: var(--accent); background: none; }.detail-heading { align-items: flex-start; flex-direction: column; }.form-grid { grid-template-columns: 1fr; }.form-grid label.wide { grid-column: auto; }.autodream-events-panel li { grid-template-columns: 1fr; gap: var(--space-1, 4px); } }
</style>
