<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { ChevronDown, FileText, FolderOpen, RefreshCw, RotateCcw, ShieldCheck } from '@lucide/vue';
import type { WorkspaceContextState } from '../../composables/useWorkspaceContext';
import {
  chooseWorkspaceDirectory,
  getDefaultWorkspace,
  inspectWorkspace,
  resetDefaultWorkspace,
  setDefaultWorkspace,
  type DefaultWorkspace,
  type WorkspaceCandidate,
  type WorkspaceStatus,
} from '../../api/desktop';
import { formatDisplayPath } from '../../utils/pathDisplay';

const props = defineProps<{
  workspace: WorkspaceStatus | null;
  state: WorkspaceContextState;
  error?: string | null;
  refreshWorkspace: () => Promise<boolean>;
}>();

const agentsOpen = ref(false);
const draftRoot = ref('');
const candidate = ref<WorkspaceCandidate | null>(null);
const inspectState = ref<'idle' | 'loading' | 'ready' | 'error'>('idle');
const inspectError = ref<string | null>(null);
const commitError = ref<string | null>(null);
const resetError = ref<string | null>(null);
const defaultWorkspace = ref<DefaultWorkspace | null>(null);
const defaultState = ref<'loading' | 'ready' | 'saving' | 'resetting' | 'error'>('loading');
const defaultError = ref<string | null>(null);
const defaultBusy = computed(() => (
  defaultState.value === 'loading'
  || defaultState.value === 'saving'
  || defaultState.value === 'resetting'
));
let inspectGeneration = 0;
const workspaceDisplayRoot = computed(() => (
  props.workspace?.root ? formatDisplayPath(props.workspace.root) : null
));

async function loadDefaultWorkspace() {
  defaultState.value = 'loading';
  defaultError.value = null;
  try {
    defaultWorkspace.value = await getDefaultWorkspace();
    defaultState.value = 'ready';
  } catch (cause) {
    defaultState.value = 'error';
    defaultError.value = cause instanceof Error ? cause.message : String(cause);
  }
}

function resetDraft() {
  inspectGeneration += 1;
  draftRoot.value = '';
  candidate.value = null;
  inspectState.value = 'idle';
  inspectError.value = null;
  commitError.value = null;
}

async function inspectDraft(root = draftRoot.value) {
  const trimmed = root.trim();
  draftRoot.value = root;
  candidate.value = null;
  inspectError.value = null;
  if (!trimmed) {
    inspectState.value = 'error';
    inspectError.value = '请输入或选择一个工作区目录。';
    return false;
  }

  const generation = ++inspectGeneration;
  inspectState.value = 'loading';
  try {
    const inspected = await inspectWorkspace(trimmed);
    if (generation !== inspectGeneration) return false;
    candidate.value = inspected;
    inspectState.value = 'ready';
    return true;
  } catch (cause) {
    if (generation !== inspectGeneration) return false;
    inspectState.value = 'error';
    inspectError.value = cause instanceof Error ? cause.message : String(cause);
    return false;
  }
}

async function chooseDirectory() {
  try {
    const selected = await chooseWorkspaceDirectory();
    if (!selected) return;
    await inspectDraft(selected);
  } catch (cause) {
    inspectState.value = 'error';
    inspectError.value = cause instanceof Error ? cause.message : String(cause);
  }
}

async function commitCandidate() {
  if (!candidate.value) return;
  commitError.value = null;
  resetError.value = null;
  defaultState.value = 'saving';
  try {
    defaultWorkspace.value = await setDefaultWorkspace(candidate.value.root);
    defaultState.value = 'ready';
    resetDraft();
  } catch (cause) {
    defaultState.value = 'error';
    commitError.value = cause instanceof Error ? cause.message : String(cause);
  }
}

async function resetToDefaultWorkspace() {
  resetError.value = null;
  commitError.value = null;
  defaultState.value = 'resetting';
  try {
    defaultWorkspace.value = await resetDefaultWorkspace();
    defaultState.value = 'ready';
    resetDraft();
  } catch (cause) {
    defaultState.value = 'error';
    resetError.value = cause instanceof Error ? cause.message : String(cause);
  }
}

onMounted(loadDefaultWorkspace);
</script>

<template>
  <div class="workspace-settings-page">
    <div class="workspace-settings-intro">
      <div class="workspace-settings-icon"><FolderOpen :size="22" /></div>
      <div>
        <p class="workspace-settings-kicker">运行时边界</p>
        <h3>工作区</h3>
        <p>默认目录只决定未显式选择时从哪里启动；当前 session 始终属于当前运行时工作区。</p>
      </div>
    </div>

    <section class="workspace-settings-card">
      <div class="workspace-settings-card-heading">
        <div>
          <p class="workspace-settings-label">默认工作区目录</p>
          <p class="workspace-settings-path">{{ defaultWorkspace ? formatDisplayPath(defaultWorkspace.root) : defaultError || '正在读取…' }}</p>
        </div>
        <span class="workspace-settings-state" :class="`workspace-settings-state-${defaultState}`">
          {{ defaultState === 'ready' ? '已保存' : defaultState === 'error' ? '操作失败' : defaultState === 'saving' ? '保存中' : defaultState === 'resetting' ? '重置中' : '读取中' }}
        </span>
      </div>
      <p v-if="defaultWorkspace?.isBuiltInDefault" class="workspace-settings-meta">Diva 内置默认目录</p>
      <div class="workspace-settings-current-actions">
        <button type="button" class="workspace-settings-refresh" :disabled="defaultBusy" @click="loadDefaultWorkspace">
          <RefreshCw :size="14" :class="{ 'animate-spin': defaultState === 'loading' }" />
          刷新默认值
        </button>
        <button
          type="button"
          class="workspace-settings-reset"
          data-testid="workspace-reset"
          :disabled="defaultBusy"
          title="恢复到用户配置目录下的 Diva workspace"
          @click="resetToDefaultWorkspace"
        >
          <RotateCcw :size="14" />
          重置为 Diva 默认目录
        </button>
      </div>
      <p v-if="resetError" class="workspace-settings-switch-error">{{ resetError }}</p>
    </section>

    <section class="workspace-settings-card">
      <div class="workspace-settings-card-heading">
        <div>
          <p class="workspace-settings-label">当前 session 所属工作区</p>
          <p class="workspace-settings-path">{{ workspaceDisplayRoot || error || '正在读取…' }}</p>
        </div>
        <span class="workspace-settings-state" :class="`workspace-settings-state-${state}`">
          {{ state === 'ready' ? '运行中' : state === 'error' ? '读取失败' : '读取中' }}
        </span>
      </div>
      <button type="button" class="workspace-settings-refresh" :disabled="state === 'loading' || state === 'refreshing'" @click="refreshWorkspace">
        <RefreshCw :size="14" :class="{ 'animate-spin': state === 'loading' || state === 'refreshing' }" />
        刷新 session 状态
      </button>
    </section>

    <section class="workspace-settings-card">
      <button type="button" class="workspace-settings-disclosure" @click="agentsOpen = !agentsOpen">
        <span class="workspace-settings-disclosure-title"><FileText :size="16" /> AGENTS.md 状态</span>
        <span class="workspace-settings-disclosure-right">
          <span>{{ workspace?.agents_md?.present ? '已发现' : '未发现' }}</span>
          <ChevronDown :size="15" :class="{ 'rotate-180': agentsOpen }" />
        </span>
      </button>
      <div v-if="agentsOpen" class="workspace-settings-agents-drawer">
        <div class="workspace-settings-readonly"><ShieldCheck :size="14" /> 只读观测；GUI 不编辑项目规则。</div>
        <dl v-if="workspace?.agents_md" class="workspace-settings-details">
          <div><dt>来源文件</dt><dd>{{ formatDisplayPath(workspace.agents_md.path) }}</dd></div>
          <div><dt>Digest</dt><dd>{{ workspace.agents_md.digest }}</dd></div>
          <div><dt>字符数</dt><dd>{{ workspace.agents_md.char_count }}</dd></div>
          <div><dt>注入状态</dt><dd>{{ workspace.agents_md.truncated ? '已按预算截断' : '完整' }}</dd></div>
        </dl>
        <p v-else class="workspace-settings-empty">当前工作区没有可观测的 AGENTS.md。</p>
      </div>
    </section>

    <section class="workspace-settings-card workspace-settings-switch-card">
      <div class="workspace-settings-card-heading">
        <div>
          <p class="workspace-settings-label">更改默认工作区目录</p>
          <p class="workspace-settings-switch-hint">选择并预检目录；保存只更新默认值，不改变当前 session 工作区。</p>
        </div>
        <FolderOpen :size="17" class="workspace-settings-switch-icon" />
      </div>
      <div class="workspace-settings-picker-row">
        <input
          v-model="draftRoot"
          class="workspace-settings-path-input"
          type="text"
          placeholder="输入绝对路径，或使用目录选择"
          :disabled="defaultBusy"
          @keyup.enter="inspectDraft()"
        />
        <button type="button" class="workspace-settings-secondary" :disabled="defaultBusy" @click="chooseDirectory">选择目录</button>
        <button type="button" class="workspace-settings-secondary" :disabled="defaultBusy" @click="inspectDraft()">
          {{ inspectState === 'loading' ? '预检中…' : '预检' }}
        </button>
      </div>
      <p v-if="inspectState === 'error'" class="workspace-settings-switch-error">{{ inspectError }}</p>
      <p v-if="commitError" class="workspace-settings-switch-error">{{ commitError }}</p>
      <div v-if="candidate" class="workspace-settings-candidate" data-testid="workspace-candidate">
        <div class="workspace-settings-candidate-heading">
          <span class="workspace-settings-candidate-status">候选已验证</span>
          <span class="workspace-settings-candidate-readable">{{ candidate.readable ? '可读' : '不可读' }}</span>
        </div>
        <dl class="workspace-settings-details">
          <div><dt>Canonical root</dt><dd>{{ formatDisplayPath(candidate.root) }}</dd></div>
          <div><dt>Workspace ID</dt><dd>{{ candidate.workspaceId }}</dd></div>
          <div><dt>AGENTS.md</dt><dd>{{ candidate.agentsMd?.present ? `已发现 · ${candidate.agentsMd.digest}` : '未发现' }}</dd></div>
        </dl>
        <button
          type="button"
          class="workspace-settings-primary"
          :disabled="defaultBusy"
          @click="commitCandidate"
        >
          {{ defaultState === 'saving' ? '保存中…' : '设为默认工作区目录' }}
        </button>
      </div>
      <p v-else class="workspace-settings-boundary">候选目录未通过预检前不会保存；保存默认值也不会改变当前 session。</p>
    </section>
  </div>
</template>

<style scoped>
.workspace-settings-page { padding: 1.5rem; max-width: 760px; margin: 0 auto; }
.workspace-settings-intro { display: flex; gap: 0.85rem; align-items: flex-start; margin-bottom: 1.25rem; }
.workspace-settings-icon { display: grid; place-items: center; width: 42px; height: 42px; border-radius: 0.8rem; background: color-mix(in srgb, var(--accent, #ec4899) 12%, transparent); color: var(--accent, #ec4899); }
.workspace-settings-kicker { margin: 0; color: var(--text-muted, #64748b); font-size: 0.68rem; }
.workspace-settings-intro h3 { margin: 0.15rem 0; color: var(--text, #1e293b); font-size: 1.2rem; }
.workspace-settings-intro p:last-child { margin: 0; color: var(--text-muted, #64748b); font-size: 0.75rem; line-height: 1.5; }
.workspace-settings-card { margin-top: 0.85rem; padding: 1rem; border: 1px solid var(--line, rgba(148, 163, 184, 0.24)); border-radius: 0.85rem; background: var(--surface, rgba(255, 255, 255, 0.62)); }
.workspace-settings-card-heading { display: flex; justify-content: space-between; gap: 1rem; align-items: flex-start; }
.workspace-settings-label { margin: 0; color: var(--text-muted, #64748b); font-size: 0.68rem; }
.workspace-settings-path { margin: 0.35rem 0 0; color: var(--text, #1e293b); font-family: ui-monospace, SFMono-Regular, monospace; font-size: 0.72rem; line-height: 1.45; overflow-wrap: anywhere; }
.workspace-settings-state { flex: 0 0 auto; padding: 0.25rem 0.45rem; border-radius: 999px; font-size: 0.62rem; }
.workspace-settings-state-ready { color: #15803d; background: #dcfce7; }
.workspace-settings-state-error { color: #b45309; background: #fef3c7; }
.workspace-settings-state-loading, .workspace-settings-state-refreshing, .workspace-settings-state-saving, .workspace-settings-state-resetting { color: #64748b; background: #f1f5f9; }
.workspace-settings-meta { display: flex; flex-direction: column; gap: 0.35rem; margin-top: 0.75rem; color: var(--text-muted, #64748b); font-size: 0.68rem; line-height: 1.4; }
.workspace-settings-current-actions { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-top: 0.85rem; }
.workspace-settings-refresh, .workspace-settings-reset { display: inline-flex; align-items: center; gap: 0.4rem; padding: 0.45rem 0.65rem; border-radius: 0.5rem; color: var(--accent, #db2777); font-size: 0.68rem; }
.workspace-settings-reset { color: var(--text-muted, #64748b); }
.workspace-settings-refresh:hover:not(:disabled), .workspace-settings-reset:hover:not(:disabled) { background: var(--nav-hover, rgba(148, 163, 184, 0.12)); }
.workspace-settings-refresh:disabled, .workspace-settings-reset:disabled { opacity: 0.55; cursor: not-allowed; }
.workspace-settings-card > .workspace-settings-refresh { margin-top: 0.85rem; }
.workspace-settings-disclosure { display: flex; justify-content: space-between; width: 100%; color: var(--text, #334155); font-size: 0.75rem; text-align: left; }
.workspace-settings-disclosure-title, .workspace-settings-disclosure-right { display: inline-flex; align-items: center; gap: 0.45rem; }
.workspace-settings-disclosure-right { color: var(--text-muted, #64748b); font-size: 0.68rem; }
.workspace-settings-agents-drawer { padding-top: 0.85rem; }
.workspace-settings-readonly { display: flex; align-items: center; gap: 0.35rem; color: var(--text-muted, #64748b); font-size: 0.67rem; }
.workspace-settings-details { display: grid; gap: 0.6rem; margin: 0.85rem 0 0; }
.workspace-settings-details div { display: grid; grid-template-columns: 72px minmax(0, 1fr); gap: 0.6rem; }
.workspace-settings-details dt { color: var(--text-muted, #64748b); font-size: 0.65rem; }
.workspace-settings-details dd { margin: 0; color: var(--text, #334155); font-family: ui-monospace, SFMono-Regular, monospace; font-size: 0.65rem; overflow-wrap: anywhere; }
.workspace-settings-empty { margin: 0.85rem 0 0; color: var(--text-muted, #64748b); font-size: 0.68rem; }
.workspace-settings-boundary { margin: 1rem 0 0; color: var(--text-muted, #64748b); font-size: 0.68rem; line-height: 1.5; }
.workspace-settings-switch-card { border-color: color-mix(in srgb, var(--accent, #ec4899) 22%, var(--line, rgba(148, 163, 184, 0.24))); }
.workspace-settings-switch-hint { margin: 0.35rem 0 0; color: var(--text-muted, #64748b); font-size: 0.68rem; line-height: 1.45; }
.workspace-settings-switch-icon { color: var(--accent, #db2777); }
.workspace-settings-picker-row { display: flex; gap: 0.45rem; margin-top: 0.9rem; }
.workspace-settings-path-input { min-width: 0; flex: 1; padding: 0.5rem 0.6rem; border: 1px solid var(--line, rgba(148, 163, 184, 0.3)); border-radius: 0.5rem; background: var(--surface-raised, rgba(255, 255, 255, 0.7)); color: var(--text, #334155); font-family: ui-monospace, SFMono-Regular, monospace; font-size: 0.68rem; }
.workspace-settings-secondary, .workspace-settings-primary { padding: 0.45rem 0.65rem; border-radius: 0.5rem; font-size: 0.68rem; white-space: nowrap; }
.workspace-settings-secondary { border: 1px solid var(--line, rgba(148, 163, 184, 0.3)); color: var(--text, #334155); }
.workspace-settings-secondary:hover:not(:disabled) { background: var(--nav-hover, rgba(148, 163, 184, 0.12)); }
.workspace-settings-primary { margin-top: 0.9rem; color: white; background: var(--accent, #db2777); }
.workspace-settings-primary:hover:not(:disabled) { filter: brightness(0.96); }
.workspace-settings-secondary:disabled, .workspace-settings-primary:disabled { opacity: 0.55; cursor: wait; }
.workspace-settings-switch-error, .workspace-settings-switch-warning { margin: 0.75rem 0 0; color: #b45309; font-size: 0.68rem; line-height: 1.45; }
.workspace-settings-candidate { margin-top: 0.9rem; padding: 0.8rem; border: 1px solid color-mix(in srgb, #22c55e 35%, var(--line, rgba(148, 163, 184, 0.24))); border-radius: 0.65rem; background: color-mix(in srgb, #22c55e 5%, transparent); }
.workspace-settings-candidate-heading { display: flex; justify-content: space-between; gap: 0.75rem; align-items: center; }
.workspace-settings-candidate-status { color: #15803d; font-size: 0.7rem; font-weight: 600; }
.workspace-settings-candidate-readable { color: #15803d; font-size: 0.65rem; }
@media (max-width: 640px) { .workspace-settings-picker-row { flex-wrap: wrap; } .workspace-settings-path-input { flex-basis: 100%; } }
</style>
