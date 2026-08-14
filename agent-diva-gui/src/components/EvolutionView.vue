<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { FilePlus2, History, RefreshCw, Save, Search, ShieldCheck, WandSparkles } from '@lucide/vue';
import {
  acceptSkillRequest,
  createSkillRequest,
  deleteSkill,
  disableSkill,
  getSkill,
  getSkillHistoryRevision,
  getSkillRequest,
  getSkills,
  listSkillHistory,
  listSkillRequests,
  rejectSkillRequest,
  updateSkill,
} from '../api/desktop';
import type {
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

type EvolutionTab = 'skills' | 'requests';
type LegacyTab = EvolutionTab | 'inbox' | 'runs' | 'audit' | 'policy';
type CountTone = 'none' | 'warning';

const props = withDefaults(defineProps<{
  initialTab?: LegacyTab;
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
}>();

const activeTab = ref<EvolutionTab>(props.initialTab === 'skills' ? 'skills' : 'requests');
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
const pendingCount = computed(() => requests.value.filter((request) => request.status === 'pending').length);
const createBaseHash = computed(() => skills.value.find((skill) => skill.slug === createSlug.value.trim())?.content_hash ?? '0');

function normalizeError(error: unknown) {
  return errorMessage(error, 'Evolution 请求失败');
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
    const next = await getSkills();
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
  await Promise.all([loadSkills(), loadRequests()]);
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
  } else {
    selectedRequestId.value = null;
    selectedRequest.value = null;
  }
}

watch(() => props.requestKey, async () => {
  activeTab.value = props.initialTab === 'skills' ? 'skills' : 'requests';
  if (props.initialProposalId) {
    await loadRequests();
    if (requests.value.some((item) => item.id === props.initialProposalId)) {
      await selectRequest(props.initialProposalId);
    }
  }
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
    </nav>

    <div class="toolbar">
      <label class="search-box"><Search :size="16" /><input v-model="search" placeholder="搜索 slug、描述或标题" /></label>
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

    <div class="workspace" :class="{ 'has-detail': selectedSlug || selectedRequestId }">
      <aside class="master-list">
        <template v-if="activeTab === 'skills'">
          <p v-if="skillsError" class="state error">加载失败：{{ skillsError }} <button @click="loadSkills">重试</button></p>
          <p v-else-if="skillsLoading && skills.length === 0" class="state">正在加载 Skill…</p>
          <p v-else-if="skills.length === 0" class="state">尚无可见 Skill。</p>
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
.evolution-shell { height: 100%; min-height: 0; display: flex; flex-direction: column; padding: 24px; gap: 14px; color: var(--text); background: radial-gradient(circle at 85% 0%, color-mix(in srgb, var(--accent) 11%, transparent), transparent 34%); }
.evolution-header, .toolbar, .detail-heading, .header-actions, .detail-actions, .panel-actions, .facts, .row-meta { display: flex; align-items: center; }
.evolution-header, .toolbar, .detail-heading { justify-content: space-between; gap: 16px; }
h1, h2, h3, p { margin: 0; } h1 { font-size: 25px; } h2 { font-size: 21px; } .evolution-header p, .detail-heading p { color: var(--text-muted); margin-top: 5px; }
.eyebrow { color: var(--accent) !important; font: 700 11px/1 var(--font-mono, monospace); letter-spacing: .15em; text-transform: uppercase; }
button, input { font: inherit; } button { cursor: pointer; } button:disabled { cursor: not-allowed; opacity: .48; }
.icon-button, .secondary, .primary, .danger, .history-toggle { display: inline-flex; align-items: center; justify-content: center; gap: 7px; border-radius: 9px; min-height: 36px; padding: 0 13px; border: 1px solid var(--border); background: var(--panel-solid); color: var(--text); }
.icon-button { width: 38px; padding: 0; } .primary { color: white; background: var(--accent); border-color: var(--accent); } .danger { color: var(--danger); border-color: color-mix(in srgb, var(--danger) 45%, var(--border)); }
.header-actions, .detail-actions, .panel-actions, .facts, .row-meta { gap: 8px; flex-wrap: wrap; }
.tabbar { display: flex; gap: 6px; padding: 5px; width: fit-content; border: 1px solid var(--border); border-radius: 12px; background: var(--panel-solid); }
.tabbar button { display: flex; align-items: center; gap: 8px; padding: 8px 14px; border: 0; border-radius: 8px; color: var(--text-muted); background: transparent; }
.tabbar button.active { color: var(--text); background: color-mix(in srgb, var(--accent) 13%, var(--panel-solid)); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 28%, transparent); }
.tabbar span { min-width: 20px; padding: 1px 6px; border-radius: 999px; background: color-mix(in srgb, var(--text) 8%, transparent); font-size: 11px; }
.search-box { flex: 1; max-width: 480px; display: flex; align-items: center; gap: 8px; min-height: 39px; padding: 0 12px; border: 1px solid var(--border); border-radius: 10px; background: var(--panel-solid); color: var(--text-muted); }
.search-box input, .form-grid input { width: 100%; border: 0; outline: 0; color: var(--text); background: transparent; }
.workspace { min-height: 0; flex: 1; display: grid; grid-template-columns: minmax(250px, 330px) minmax(0, 1fr); overflow: hidden; border: 1px solid var(--border); border-radius: 14px; background: color-mix(in srgb, var(--panel-solid) 92%, transparent); }
.master-list { min-height: 0; overflow: auto; padding: 9px; border-right: 1px solid var(--border); }
.list-row { width: 100%; display: grid; grid-template-columns: 1fr auto; gap: 5px 9px; padding: 12px; border: 1px solid transparent; border-radius: 10px; background: transparent; color: var(--text); text-align: left; }
.list-row:hover, .list-row.selected { background: color-mix(in srgb, var(--accent) 8%, transparent); border-color: color-mix(in srgb, var(--accent) 24%, transparent); }
.row-title { min-width: 0; overflow: hidden; font-weight: 650; text-overflow: ellipsis; white-space: nowrap; }.row-description { grid-column: 1 / -1; overflow: hidden; color: var(--text-muted); font-size: 12px; text-overflow: ellipsis; white-space: nowrap; }.row-meta { grid-column: 1 / -1; color: var(--text-muted); font-size: 11px; }
.source, .status { width: fit-content; padding: 2px 7px; border-radius: 999px; font: 700 10px/1.5 var(--font-mono, monospace); text-transform: uppercase; background: color-mix(in srgb, var(--accent) 12%, transparent); color: var(--accent); }.source.builtin { color: var(--text-muted); background: color-mix(in srgb, var(--text) 7%, transparent); }.status.stale, .status.rejected { color: var(--danger); background: color-mix(in srgb, var(--danger) 10%, transparent); }.status.accepted { color: var(--success); background: color-mix(in srgb, var(--success) 10%, transparent); }
.detail-pane { min-height: 0; overflow: auto; padding: 20px; }.detail-empty, .state { padding: 32px 16px; color: var(--text-muted); text-align: center; }.state.error, .error-banner { color: var(--danger); }.state button { border: 0; color: var(--accent); background: none; }
.facts { margin: 13px 0; color: var(--text-muted); font: 11px var(--font-mono, monospace); }.facts > span { padding: 4px 8px; border-radius: 7px; background: color-mix(in srgb, var(--text) 6%, transparent); }.muted { color: var(--text-muted); text-decoration: line-through; }
.editor-frame { height: min(54vh, 560px); min-height: 260px; overflow: hidden; border: 1px solid var(--border); border-radius: 11px; background: color-mix(in srgb, var(--panel-solid) 88%, transparent); }.history-toggle { margin-top: 12px; }.history-panel, .evidence-panel, .create-panel { padding: 14px; border: 1px solid var(--border); border-radius: 11px; background: var(--panel-solid); }.history-panel { margin-top: 8px; }.history-panel button { display: block; width: 100%; padding: 8px; border: 0; color: var(--text-muted); background: transparent; text-align: left; }.history-panel pre, .evidence-panel pre { overflow: auto; padding: 10px; border-radius: 8px; background: color-mix(in srgb, var(--text) 5%, transparent); white-space: pre-wrap; }.evidence-panel { margin-top: 12px; display: grid; gap: 9px; }.stale-notice, .error-banner { padding: 10px 12px; border: 1px solid color-mix(in srgb, var(--danger) 35%, transparent); border-radius: 9px; background: color-mix(in srgb, var(--danger) 7%, transparent); }
.create-panel { display: grid; gap: 12px; }.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }.form-grid label { display: grid; gap: 5px; color: var(--text-muted); font-size: 12px; }.form-grid label.wide { grid-column: 1 / -1; }.form-grid input { min-height: 36px; padding: 0 10px; border: 1px solid var(--border); border-radius: 8px; }.create-editor { height: 260px; }.panel-actions { justify-content: flex-end; }.mobile-back { display: none; }
@media (max-width: 760px) { .evolution-shell { padding: 14px; }.evolution-header { align-items: flex-start; }.evolution-header > div:first-child p:last-child { display: none; }.workspace { display: block; }.detail-pane { display: none; height: 100%; }.workspace.has-detail .master-list { display: none; }.workspace.has-detail .detail-pane { display: block; }.mobile-back { display: inline-flex; margin-bottom: 12px; border: 0; color: var(--accent); background: none; }.detail-heading { align-items: flex-start; flex-direction: column; }.form-grid { grid-template-columns: 1fr; }.form-grid label.wide { grid-column: auto; } }
</style>
