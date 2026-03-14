<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Bot, CircleAlert, Globe, Plus, RefreshCcw, Trash2, Upload } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

import {
  createMcp,
  deleteMcp,
  getMcps,
  isTauriRuntime,
  refreshMcpStatus,
  setMcpEnabled,
  updateMcp,
  type McpServerDto,
  type McpServerPayload,
} from '../../api/desktop';

const { t } = useI18n();

type Transport = 'stdio' | 'http';

interface FormState {
  originalName: string;
  name: string;
  enabled: boolean;
  transport: Transport;
  command: string;
  args: string[];
  env: Array<{ key: string; value: string }>;
  url: string;
  tool_timeout: number;
}

const mcps = ref<McpServerDto[]>([]);
const loading = ref(false);
const saving = ref(false);
const error = ref('');
const importMessage = ref('');
const previewMode = computed(() => !isTauriRuntime());
const showEditor = ref(false);
const rawJson = ref('');
const editorMode = ref<'create' | 'edit'>('create');
const form = ref<FormState>(blankForm());

function blankForm(): FormState {
  return {
    originalName: '',
    name: '',
    enabled: true,
    transport: 'stdio',
    command: '',
    args: [],
    env: [],
    url: '',
    tool_timeout: 30,
  };
}

function dtoToForm(dto: McpServerDto): FormState {
  return {
    originalName: dto.name,
    name: dto.name,
    enabled: dto.enabled,
    transport: dto.transport === 'http' ? 'http' : 'stdio',
    command: dto.command,
    args: [...dto.args],
    env: Object.entries(dto.env).map(([key, value]) => ({ key, value })),
    url: dto.url,
    tool_timeout: dto.tool_timeout,
  };
}

function formToPayload(state: FormState): McpServerPayload {
  const env = Object.fromEntries(
    state.env
      .map((item) => [item.key.trim(), item.value] as const)
      .filter(([key]) => key.length > 0)
  );

  return {
    name: state.name.trim(),
    enabled: state.enabled,
    command: state.transport === 'stdio' ? state.command.trim() : '',
    args: state.transport === 'stdio' ? state.args.filter((item) => item.trim().length > 0) : [],
    env: state.transport === 'stdio' ? env : {},
    url: state.transport === 'http' ? state.url.trim() : '',
    tool_timeout: Number(state.tool_timeout) || 30,
  };
}

function syncRawJsonFromForm() {
  rawJson.value = JSON.stringify(formToPayload(form.value), null, 2);
}

function normalizeSinglePayload(input: any): McpServerPayload {
  return {
    name: String(input.name ?? '').trim(),
    enabled: input.enabled !== false,
    command: String(input.command ?? ''),
    args: Array.isArray(input.args) ? input.args.map((item: unknown) => String(item)) : [],
    env: typeof input.env === 'object' && input.env ? input.env : {},
    url: String(input.url ?? ''),
    tool_timeout: Number(input.tool_timeout ?? input.toolTimeout ?? 30) || 30,
  };
}

function applyRawJsonToForm() {
  const payload = normalizeSinglePayload(JSON.parse(rawJson.value));
  form.value = {
    ...form.value,
    name: payload.name,
    enabled: payload.enabled,
    transport: payload.url ? 'http' : 'stdio',
    command: payload.command,
    args: payload.args,
    env: Object.entries(payload.env).map(([key, value]) => ({ key, value: String(value) })),
    url: payload.url,
    tool_timeout: payload.tool_timeout,
  };
}

function extractImportPayloads(input: any): McpServerPayload[] {
  if (input?.tools?.mcpServers && typeof input.tools.mcpServers === 'object') {
    return extractImportPayloads(input.tools.mcpServers);
  }
  if (input?.tools?.mcp_servers && typeof input.tools.mcp_servers === 'object') {
    return extractImportPayloads(input.tools.mcp_servers);
  }
  if (input?.name && (input?.command || input?.url)) {
    return [normalizeSinglePayload(input)];
  }
  if (typeof input === 'object' && input) {
    return Object.entries(input).map(([name, value]: [string, any]) =>
      normalizeSinglePayload({ name, ...value })
    );
  }
  throw new Error(t('mcp.invalidJson'));
}

async function refreshList() {
  if (previewMode.value) {
    mcps.value = [];
    error.value = '';
    return;
  }

  loading.value = true;
  error.value = '';
  try {
    mcps.value = await getMcps();
  } catch (err) {
    error.value = String(err);
  } finally {
    loading.value = false;
  }
}

function openCreate() {
  editorMode.value = 'create';
  form.value = blankForm();
  syncRawJsonFromForm();
  showEditor.value = true;
}

function openEdit(dto: McpServerDto) {
  editorMode.value = 'edit';
  form.value = dtoToForm(dto);
  syncRawJsonFromForm();
  showEditor.value = true;
}

function closeEditor() {
  showEditor.value = false;
}

async function submitForm() {
  if (previewMode.value) {
    return;
  }
  saving.value = true;
  error.value = '';
  try {
    const payload = formToPayload(form.value);
    if (editorMode.value === 'create') {
      await createMcp(payload);
    } else {
      await updateMcp(form.value.originalName, payload);
    }
    await refreshList();
    closeEditor();
  } catch (err) {
    error.value = String(err);
  } finally {
    saving.value = false;
  }
}

async function removeMcp(name: string) {
  if (previewMode.value || !window.confirm(t('mcp.deleteConfirm', { name }))) {
    return;
  }
  try {
    await deleteMcp(name);
    await refreshList();
  } catch (err) {
    error.value = String(err);
  }
}

async function toggleMcp(item: McpServerDto) {
  if (previewMode.value) {
    return;
  }
  try {
    await setMcpEnabled(item.name, !item.enabled);
    await refreshList();
  } catch (err) {
    error.value = String(err);
  }
}

async function refreshOne(name: string) {
  if (previewMode.value) {
    return;
  }
  try {
    await refreshMcpStatus(name);
    await refreshList();
  } catch (err) {
    error.value = String(err);
  }
}

async function onImportJson(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) {
    return;
  }

  try {
    const payloads = extractImportPayloads(JSON.parse(await file.text()));
    const existing = new Set(mcps.value.map((item) => item.name));
    for (const payload of payloads) {
      if (existing.has(payload.name)) {
        await updateMcp(payload.name, payload);
      } else {
        await createMcp(payload);
      }
    }
    importMessage.value = t('mcp.importSuccess');
    await refreshList();
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    input.value = '';
  }
}

function addArg() {
  form.value.args.push('');
  syncRawJsonFromForm();
}

function addEnv() {
  form.value.env.push({ key: '', value: '' });
  syncRawJsonFromForm();
}

function stateClass(state: string) {
  if (state === 'connected') return 'bg-emerald-100 text-emerald-700';
  if (state === 'degraded') return 'bg-amber-100 text-amber-700';
  if (state === 'disabled') return 'bg-slate-200 text-slate-700';
  return 'bg-rose-100 text-rose-700';
}

function stateLabel(state: string) {
  if (state === 'connected') return t('mcp.stateConnected');
  if (state === 'degraded') return t('mcp.stateDegraded');
  if (state === 'disabled') return t('mcp.stateDisabled');
  return t('mcp.stateInvalid');
}

const activeMcps = computed(() => mcps.value.filter((item) => item.enabled));
const onlineCount = computed(() => mcps.value.filter((item) => item.status.connected).length);
const degradedCount = computed(() => mcps.value.filter((item) => item.enabled && !item.status.connected).length);
const disabledCount = computed(() => mcps.value.filter((item) => !item.enabled).length);

onMounted(refreshList);
</script>

<template>
  <section class="space-y-6">
    <div class="grid grid-cols-2 xl:grid-cols-4 gap-4">
      <div class="rounded-2xl bg-white border border-gray-100 p-4">
        <div class="text-xs text-gray-500">{{ t('mcp.total') }}</div>
        <div class="mt-2 text-2xl font-bold text-gray-900">{{ mcps.length }}</div>
      </div>
      <div class="rounded-2xl bg-white border border-emerald-100 p-4">
        <div class="text-xs text-emerald-600">{{ t('mcp.online') }}</div>
        <div class="mt-2 text-2xl font-bold text-emerald-700">{{ onlineCount }}</div>
      </div>
      <div class="rounded-2xl bg-white border border-amber-100 p-4">
        <div class="text-xs text-amber-600">{{ t('mcp.degraded') }}</div>
        <div class="mt-2 text-2xl font-bold text-amber-700">{{ degradedCount }}</div>
      </div>
      <div class="rounded-2xl bg-white border border-slate-200 p-4">
        <div class="text-xs text-slate-500">{{ t('mcp.disabled') }}</div>
        <div class="mt-2 text-2xl font-bold text-slate-700">{{ disabledCount }}</div>
      </div>
    </div>

    <div class="rounded-2xl bg-white border border-gray-100 p-5 space-y-4">
      <div>
        <h4 class="text-base font-semibold text-gray-800">{{ t('mcp.activeTitle') }}</h4>
        <p class="text-sm text-gray-500">{{ t('mcp.activeDesc') }}</p>
      </div>
      <div v-if="activeMcps.length === 0" class="text-sm text-gray-500">{{ t('mcp.noActive') }}</div>
      <div v-else class="grid grid-cols-1 xl:grid-cols-2 gap-3">
        <div
          v-for="item in activeMcps"
          :key="`active-${item.name}`"
          class="rounded-xl border border-gray-100 bg-gray-50 px-4 py-3 space-y-2"
        >
          <div class="flex items-center justify-between gap-2">
            <div class="flex items-center gap-2 min-w-0">
              <Bot :size="15" class="text-amber-600" />
              <span class="font-semibold text-gray-800 truncate">{{ item.name }}</span>
            </div>
            <span class="px-2 py-0.5 rounded-full text-[10px] font-semibold uppercase tracking-wide" :class="stateClass(item.status.state)">
              {{ stateLabel(item.status.state) }}
            </span>
          </div>
          <div class="text-xs text-gray-500 break-all">
            {{ item.transport === 'http' ? item.url : `${item.command} ${item.args.join(' ')}` }}
          </div>
          <div class="text-xs text-gray-400">{{ t('mcp.tools') }}: {{ item.status.tool_count }}</div>
        </div>
      </div>
    </div>

    <div class="rounded-2xl bg-white border border-gray-100 p-5 space-y-4">
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div class="space-y-1">
          <h4 class="text-base font-semibold text-gray-800">{{ t('mcp.title') }}</h4>
          <p class="text-sm text-gray-500">{{ t('mcp.desc') }}</p>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <button class="inline-flex items-center gap-2 px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-700 hover:bg-gray-50" :disabled="loading" @click="refreshList">
            <RefreshCcw :size="14" />
            {{ t('mcp.refresh') }}
          </button>
          <label class="inline-flex items-center gap-2 px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-700 hover:bg-gray-50 cursor-pointer" :class="{ 'pointer-events-none opacity-60': previewMode }">
            <Upload :size="14" />
            {{ t('mcp.importJson') }}
            <input class="hidden" type="file" accept=".json,application/json" :disabled="previewMode" @change="onImportJson" />
          </label>
          <button class="inline-flex items-center gap-2 px-3 py-2 text-xs rounded-lg bg-amber-600 text-white hover:bg-amber-700" :disabled="previewMode" @click="openCreate">
            <Plus :size="14" />
            {{ t('mcp.add') }}
          </button>
        </div>
      </div>

      <div class="rounded-xl border border-dashed border-gray-200 bg-gray-50/80 p-3 text-xs text-gray-500">
        <p>{{ t('mcp.importHint') }}</p>
        <p v-if="previewMode" class="mt-2 text-amber-600">{{ t('mcp.previewOnly') }}</p>
      </div>

      <div v-if="loading" class="text-sm text-gray-500">{{ t('mcp.refresh') }}...</div>
      <div v-else-if="mcps.length === 0" class="text-sm text-gray-500">{{ t('mcp.empty') }}</div>
      <div v-else class="space-y-3">
        <div v-for="item in mcps" :key="item.name" class="rounded-xl border border-gray-100 bg-gray-50 px-4 py-4 space-y-3">
          <div class="flex flex-wrap items-start justify-between gap-3">
            <div class="min-w-0 space-y-2">
              <div class="flex flex-wrap items-center gap-2">
                <span class="text-sm font-semibold text-gray-800">{{ item.name }}</span>
                <span class="px-2 py-0.5 rounded-full text-[10px] font-semibold uppercase tracking-wide" :class="stateClass(item.status.state)">
                  {{ stateLabel(item.status.state) }}
                </span>
                <span class="px-2 py-0.5 rounded-full text-[10px] font-semibold uppercase tracking-wide bg-sky-100 text-sky-700">
                  {{ item.transport === 'http' ? t('mcp.transportHttp') : t('mcp.transportStdio') }}
                </span>
              </div>
              <div class="text-xs text-gray-500 break-all">
                {{ item.transport === 'http' ? item.url : `${item.command} ${item.args.join(' ')}` }}
              </div>
              <div class="flex flex-wrap gap-x-4 gap-y-1 text-[11px] text-gray-400">
                <span>{{ t('mcp.tools') }}: {{ item.status.tool_count }}</span>
                <span>{{ t('mcp.timeout') }}: {{ item.tool_timeout }}</span>
                <span v-if="item.status.checked_at">{{ t('mcp.checkedAt') }}: {{ item.status.checked_at }}</span>
              </div>
              <div v-if="item.status.error" class="flex items-start gap-2 text-xs text-rose-600">
                <CircleAlert :size="14" class="mt-0.5 shrink-0" />
                <span class="break-words">{{ item.status.error }}</span>
              </div>
            </div>

            <div class="flex flex-wrap items-center gap-2">
              <button class="px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-700 hover:bg-white" :disabled="previewMode" @click="refreshOne(item.name)">
                {{ t('mcp.test') }}
              </button>
              <button class="px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-700 hover:bg-white" :disabled="previewMode" @click="openEdit(item)">
                {{ t('mcp.edit') }}
              </button>
              <button class="px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-700 hover:bg-white" :disabled="previewMode" @click="toggleMcp(item)">
                {{ item.enabled ? t('mcp.toggleOff') : t('mcp.toggleOn') }}
              </button>
              <button class="inline-flex items-center gap-1 px-3 py-2 text-xs rounded-lg border border-rose-200 text-rose-700 hover:bg-rose-50" :disabled="previewMode" @click="removeMcp(item.name)">
                <Trash2 :size="14" />
                {{ t('mcp.delete') }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <p v-if="importMessage" class="text-xs text-emerald-600">{{ importMessage }}</p>
      <p v-if="error" class="text-xs text-red-600 break-words">{{ error }}</p>
    </div>

    <div v-if="showEditor" class="fixed inset-0 z-20 bg-slate-900/40 backdrop-blur-sm flex items-center justify-center p-4">
      <div class="w-full max-w-5xl max-h-[90vh] overflow-y-auto rounded-2xl bg-white shadow-2xl border border-gray-100 p-6 space-y-6">
        <div class="flex items-start justify-between gap-3">
          <div>
            <h4 class="text-lg font-semibold text-gray-900">{{ editorMode === 'create' ? t('mcp.formCreate') : t('mcp.formEdit') }}</h4>
            <p class="text-sm text-gray-500">{{ t('mcp.currentUsing') }}</p>
          </div>
          <button class="px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-600" @click="closeEditor">
            {{ t('mcp.cancel') }}
          </button>
        </div>

        <div class="grid grid-cols-1 xl:grid-cols-2 gap-6">
          <div class="space-y-4">
            <label class="block space-y-1">
              <span class="text-xs font-medium text-gray-600">{{ t('mcp.name') }}</span>
              <input v-model="form.name" class="w-full rounded-xl border border-gray-200 px-3 py-2 text-sm" @input="syncRawJsonFromForm" />
            </label>

            <div class="grid grid-cols-2 gap-4">
              <label class="block space-y-1">
                <span class="text-xs font-medium text-gray-600">{{ t('mcp.transport') }}</span>
                <select v-model="form.transport" class="w-full rounded-xl border border-gray-200 px-3 py-2 text-sm" @change="syncRawJsonFromForm">
                  <option value="stdio">{{ t('mcp.transportStdio') }}</option>
                  <option value="http">{{ t('mcp.transportHttp') }}</option>
                </select>
              </label>
              <label class="flex items-center gap-2 pt-6 text-sm text-gray-700">
                <input v-model="form.enabled" type="checkbox" @change="syncRawJsonFromForm" />
                <span>{{ t('mcp.enabled') }}</span>
              </label>
            </div>

            <template v-if="form.transport === 'stdio'">
              <label class="block space-y-1">
                <span class="text-xs font-medium text-gray-600">{{ t('mcp.command') }}</span>
                <input v-model="form.command" class="w-full rounded-xl border border-gray-200 px-3 py-2 text-sm" @input="syncRawJsonFromForm" />
              </label>
              <div class="space-y-2">
                <div class="flex items-center justify-between">
                  <span class="text-xs font-medium text-gray-600">{{ t('mcp.args') }}</span>
                  <button class="text-xs text-amber-700" @click="addArg">{{ t('mcp.addArg') }}</button>
                </div>
                <div v-for="(_, index) in form.args" :key="`arg-${index}`" class="flex items-center gap-2">
                  <input v-model="form.args[index]" class="flex-1 rounded-xl border border-gray-200 px-3 py-2 text-sm" :placeholder="t('mcp.argPlaceholder')" @input="syncRawJsonFromForm" />
                  <button class="text-xs text-rose-600" @click="form.args.splice(index, 1); syncRawJsonFromForm()">×</button>
                </div>
              </div>
              <div class="space-y-2">
                <div class="flex items-center justify-between">
                  <span class="text-xs font-medium text-gray-600">{{ t('mcp.env') }}</span>
                  <button class="text-xs text-amber-700" @click="addEnv">{{ t('mcp.addEnv') }}</button>
                </div>
                <div v-for="(item, index) in form.env" :key="`env-${index}`" class="grid grid-cols-[1fr,1fr,auto] gap-2">
                  <input v-model="item.key" class="rounded-xl border border-gray-200 px-3 py-2 text-sm" :placeholder="t('mcp.envKeyPlaceholder')" @input="syncRawJsonFromForm" />
                  <input v-model="item.value" class="rounded-xl border border-gray-200 px-3 py-2 text-sm" :placeholder="t('mcp.envValuePlaceholder')" @input="syncRawJsonFromForm" />
                  <button class="text-xs text-rose-600" @click="form.env.splice(index, 1); syncRawJsonFromForm()">×</button>
                </div>
              </div>
            </template>

            <template v-else>
              <label class="block space-y-1">
                <span class="text-xs font-medium text-gray-600">{{ t('mcp.url') }}</span>
                <div class="relative">
                  <Globe :size="14" class="absolute left-3 top-3 text-gray-400" />
                  <input v-model="form.url" class="w-full rounded-xl border border-gray-200 pl-9 pr-3 py-2 text-sm" @input="syncRawJsonFromForm" />
                </div>
              </label>
            </template>

            <label class="block space-y-1">
              <span class="text-xs font-medium text-gray-600">{{ t('mcp.timeout') }}</span>
              <input v-model.number="form.tool_timeout" type="number" min="1" class="w-full rounded-xl border border-gray-200 px-3 py-2 text-sm" @input="syncRawJsonFromForm" />
            </label>
          </div>

          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <span class="text-xs font-medium text-gray-600">{{ t('mcp.editJson') }}</span>
              <div class="flex items-center gap-2">
                <button class="px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-700" @click="syncRawJsonFromForm">{{ t('mcp.syncJson') }}</button>
                <button class="px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-700" @click="applyRawJsonToForm">{{ t('mcp.applyJson') }}</button>
              </div>
            </div>
            <textarea v-model="rawJson" class="w-full min-h-[420px] rounded-2xl border border-gray-200 bg-slate-950 text-slate-100 p-4 text-xs font-mono" spellcheck="false" />
          </div>
        </div>

        <div class="flex items-center justify-end gap-3">
          <button class="px-4 py-2 text-sm rounded-xl border border-gray-200 text-gray-700" @click="closeEditor">{{ t('mcp.cancel') }}</button>
          <button class="px-4 py-2 text-sm rounded-xl bg-amber-600 text-white hover:bg-amber-700 disabled:opacity-60" :disabled="saving" @click="submitForm">
            {{ saving ? t('mcp.saving') : t('mcp.save') }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>
