<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import {
  Bot,
  CircleAlert,
  Globe,
  Plus,
  RefreshCcw,
  Trash2,
  Upload,
  Download,
  ChevronRight,
  Search,
  Copy,
  Check,
  X,
  Wrench,
  Power,
  Loader2,
} from 'lucide-vue-next';
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
import { appConfirm } from '../../utils/appDialog';
import { showAppToast } from '../../utils/appToast';

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

// 核心状态
const mcps = ref<McpServerDto[]>([]);
const loading = ref(false);
const saving = ref(false);
const error = ref('');
const importMessage = ref('');
const previewMode = computed(() => !isTauriRuntime());

// UI交互状态（参考openakita的细粒度状态管理）
const expandedServer = ref<string | null>(null);
const busyServer = ref<string | null>(null); // 操作锁：防止并发操作
const showEditor = ref(false);
const rawJson = ref('');
const editorMode = ref<'create' | 'edit'>('create');
const form = ref<FormState>(blankForm());

// 搜索和筛选状态
const searchQuery = ref('');
const statusFilter = ref<'all' | 'enabled' | 'disabled' | 'connected' | 'degraded'>('all');
const sortByRef = ref<'name' | 'status' | 'time'>('name');

// 复制状态
const copiedName = ref<string | null>(null);

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
    args: state.transport === 'stdio' ? parseArgsSmart(state.args) : [],
    env: state.transport === 'stdio' ? env : {},
    url: state.transport === 'http' ? state.url.trim() : '',
    tool_timeout: Number(state.tool_timeout) || 30,
  };
}

// 智能参数解析（参考openakita的parseArgs函数）
function parseArgsSmart(args: string[]): string[] {
  const result: string[] = [];
  for (const arg of args) {
    if (arg.trim().length === 0) continue;
    // 支持空格分隔的多参数
    const parsed = parseSingleArgLine(arg);
    result.push(...parsed);
  }
  return result;
}

function parseSingleArgLine(input: string): string[] {
  const result: string[] = [];
  let current = '';
  let inQuotes = false;
  let quoteChar = '';

  for (let i = 0; i < input.length; i++) {
    const char = input[i];

    if ((char === '"' || char === "'") && !inQuotes) {
      inQuotes = true;
      quoteChar = char;
    } else if (char === quoteChar && inQuotes) {
      inQuotes = false;
      quoteChar = '';
    } else if (char === ' ' && !inQuotes) {
      if (current.trim().length > 0) {
        result.push(current.trim());
        current = '';
      }
    } else {
      current += char;
    }
  }

  if (current.trim().length > 0) {
    result.push(current.trim());
  }

  return result;
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
  try {
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
  } catch (e) {
    error.value = t('mcp.invalidJson');
  }
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

// 过滤和排序后的列表
const filteredMcps = computed(() => {
  let result = [...mcps.value];

  // 搜索过滤
  if (searchQuery.value.trim()) {
    const query = searchQuery.value.toLowerCase();
    result = result.filter(
      (item) =>
        item.name.toLowerCase().includes(query) ||
        item.command.toLowerCase().includes(query) ||
        item.url.toLowerCase().includes(query)
    );
  }

  // 状态过滤
  if (statusFilter.value !== 'all') {
    switch (statusFilter.value) {
      case 'enabled':
        result = result.filter((item) => item.enabled);
        break;
      case 'disabled':
        result = result.filter((item) => !item.enabled);
        break;
      case 'connected':
        result = result.filter((item) => item.status.connected);
        break;
      case 'degraded':
        result = result.filter((item) => item.enabled && !item.status.connected);
        break;
    }
  }

  // 排序
  switch (sortByRef.value) {
    case 'name':
      result.sort((a, b) => a.name.localeCompare(b.name));
      break;
    case 'status':
      result.sort((a, b) => {
        const order: Record<string, number> = { connected: 0, degraded: 1, disabled: 2, invalid: 3 };
        return (order[a.status.state] ?? 99) - (order[b.status.state] ?? 99);
      });
      break;
    case 'time':
      result.sort((a, b) => {
        if (!a.status.checked_at) return 1;
        if (!b.status.checked_at) return -1;
        return new Date(b.status.checked_at).getTime() - new Date(a.status.checked_at).getTime();
      });
      break;
  }

  return result;
});

// 统计计算
const onlineCount = computed(() => mcps.value.filter((item) => item.status.connected).length);
const degradedCount = computed(() =>
  mcps.value.filter((item) => item.enabled && !item.status.connected).length
);
const disabledCount = computed(() => mcps.value.filter((item) => !item.enabled).length);

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
    showAppToast(t('mcp.operationFailed'), 'error');
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
  if (previewMode.value) return;

  // 名称验证
  const name = form.value.name.trim();
  if (!name) {
    error.value = t('mcp.nameRequired');
    showAppToast(t('mcp.nameRequired'), 'error');
    return;
  }
  if (!/^[a-zA-Z0-9_-]+$/.test(name)) {
    error.value = t('mcp.nameInvalid');
    showAppToast(t('mcp.nameInvalid'), 'error');
    return;
  }

  saving.value = true;
  error.value = '';
  try {
    const payload = formToPayload(form.value);
    if (editorMode.value === 'create') {
      await createMcp(payload);
      showAppToast(t('mcp.operationSuccess'), 'success');
    } else {
      await updateMcp(form.value.originalName, payload);
      showAppToast(t('mcp.operationSuccess'), 'success');
    }
    await refreshList();
    closeEditor();
  } catch (err) {
    error.value = String(err);
    showAppToast(t('mcp.operationFailed'), 'error');
  } finally {
    saving.value = false;
  }
}

async function removeMcp(name: string) {
  if (previewMode.value) return;
  if (busyServer.value) return; // 操作锁

  if (!(await appConfirm(t('mcp.deleteConfirm', { name })))) return;

  busyServer.value = name;
  try {
    await deleteMcp(name);
    showAppToast(t('mcp.operationSuccess'), 'success');
    await refreshList();
  } catch (err) {
    error.value = String(err);
    showAppToast(t('mcp.operationFailed'), 'error');
  } finally {
    busyServer.value = null;
  }
}

async function toggleMcp(item: McpServerDto) {
  if (previewMode.value) return;
  if (busyServer.value) return;

  busyServer.value = item.name;
  try {
    await setMcpEnabled(item.name, !item.enabled);
    showAppToast(t('mcp.operationSuccess'), 'success');
    await refreshList();
  } catch (err) {
    error.value = String(err);
    showAppToast(t('mcp.operationFailed'), 'error');
  } finally {
    busyServer.value = null;
  }
}

async function refreshOne(name: string) {
  if (previewMode.value) return;
  if (busyServer.value) return;

  busyServer.value = name;
  try {
    await refreshMcpStatus(name);
    await refreshList();
  } catch (err) {
    error.value = String(err);
    showAppToast(t('mcp.operationFailed'), 'error');
  } finally {
    busyServer.value = null;
  }
}

// 展开/折叠服务器详情
function toggleExpand(name: string) {
  expandedServer.value = expandedServer.value === name ? null : name;
}

// 复制命令
async function copyCommand(item: McpServerDto) {
  const cmd =
    item.transport === 'http'
      ? item.url
      : `${item.command} ${item.args.join(' ')}`;
  try {
    await navigator.clipboard.writeText(cmd);
    copiedName.value = item.name;
    showAppToast(t('mcp.copied'), 'success');
    setTimeout(() => {
      copiedName.value = null;
    }, 2000);
  } catch {
    showAppToast(t('mcp.copyFailed'), 'error');
  }
}

async function onImportJson(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;

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
    showAppToast(t('mcp.importSuccess'), 'success');
    await refreshList();
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
    showAppToast(t('mcp.invalidJson'), 'error');
  } finally {
    input.value = '';
  }
}

function exportToJson() {
  const exportData = {
    tools: {
      mcpServers: mcps.value.reduce(
        (acc, item) => ({
          ...acc,
          [item.name]: {
            command: item.command,
            args: item.args,
            env: item.env,
            url: item.url,
            tool_timeout: item.tool_timeout,
            enabled: item.enabled,
          },
        }),
        {}
      ),
    },
  };

  const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = 'mcp-servers.json';
  a.click();
  URL.revokeObjectURL(url);
  showAppToast(t('mcp.exportSuccess'), 'success');
}

function addArg() {
  form.value.args.push('');
  syncRawJsonFromForm();
}

function addEnv() {
  form.value.env.push({ key: '', value: '' });
  syncRawJsonFromForm();
}

// 状态徽章样式（使用设计系统的CSS变量）
function stateClass(state: string) {
  switch (state) {
    case 'connected':
      return 'mcp-status-badge mcp-status-connected';
    case 'degraded':
      return 'mcp-status-badge mcp-status-degraded';
    case 'disabled':
      return 'mcp-status-badge mcp-status-disabled';
    default:
      return 'mcp-status-badge mcp-status-invalid';
  }
}

function stateLabel(state: string) {
  switch (state) {
    case 'connected':
      return t('mcp.stateConnected');
    case 'degraded':
      return t('mcp.stateDegraded');
    case 'disabled':
      return t('mcp.stateDisabled');
    default:
      return t('mcp.stateInvalid');
  }
}

// 清除搜索
function clearSearch() {
  searchQuery.value = '';
  statusFilter.value = 'all';
}

onMounted(refreshList);
</script>

<template>
  <section class="space-y-5">
    <!-- 状态卡片区域（简化DOM结构） -->
    <div class="grid grid-cols-2 xl:grid-cols-4 gap-3">
      <div class="stat-card stat-card-total">
        <div class="stat-label">{{ t('mcp.total') }}</div>
        <div class="stat-value">{{ mcps.length }}</div>
      </div>
      <div class="stat-card stat-card-online">
        <div class="stat-label">{{ t('mcp.online') }}</div>
        <div class="stat-value">{{ onlineCount }}</div>
      </div>
      <div class="stat-card stat-card-degraded">
        <div class="stat-label">{{ t('mcp.degraded') }}</div>
        <div class="stat-value">{{ degradedCount }}</div>
      </div>
      <div class="stat-card stat-card-disabled">
        <div class="stat-label">{{ t('mcp.disabled') }}</div>
        <div class="stat-value">{{ disabledCount }}</div>
      </div>
    </div>

    <!-- 主列表区域 -->
    <div class="mcp-list-panel">
      <!-- 头部工具栏 -->
      <div class="list-header">
        <div class="header-title">
          <h4 class="title-text">{{ t('mcp.title') }}</h4>
          <p class="title-desc">{{ t('mcp.desc') }}</p>
        </div>

        <div class="header-actions">
          <button
            class="action-btn action-btn-secondary"
            :disabled="loading"
            @click="refreshList"
          >
            <RefreshCcw :size="14" :class="{ 'animate-spin': loading }" />
            {{ t('mcp.refresh') }}
          </button>

          <label class="action-btn action-btn-secondary cursor-pointer">
            <Upload :size="14" />
            {{ t('mcp.importJson') }}
            <input
              class="hidden"
              type="file"
              accept=".json,application/json"
              :disabled="previewMode"
              @change="onImportJson"
            />
          </label>

          <button
            class="action-btn action-btn-secondary"
            :disabled="mcps.length === 0"
            @click="exportToJson"
          >
            <Download :size="14" />
            {{ t('mcp.exportJson') }}
          </button>

          <button
            class="action-btn action-btn-primary"
            :disabled="previewMode"
            @click="openCreate"
          >
            <Plus :size="14" />
            {{ t('mcp.add') }}
          </button>
        </div>
      </div>

      <!-- 搜索和筛选栏 -->
      <div class="filter-bar">
        <div class="search-input-wrapper">
          <Search :size="14" class="search-icon" />
          <input
            v-model="searchQuery"
            type="text"
            class="search-input"
            :placeholder="t('mcp.searchPlaceholder')"
          />
          <button
            v-if="searchQuery"
            class="clear-search-btn"
            @click="clearSearch"
          >
            <X :size="12" />
          </button>
        </div>

        <select v-model="statusFilter" class="filter-select">
          <option value="all">{{ t('mcp.filterAll') }}</option>
          <option value="enabled">{{ t('mcp.filterEnabled') }}</option>
          <option value="disabled">{{ t('mcp.filterDisabled') }}</option>
          <option value="connected">{{ t('mcp.filterConnected') }}</option>
          <option value="degraded">{{ t('mcp.filterDegraded') }}</option>
        </select>

        <select v-model="sortByRef" class="filter-select">
          <option value="name">{{ t('mcp.sortByName') }}</option>
          <option value="status">{{ t('mcp.sortByStatus') }}</option>
          <option value="time">{{ t('mcp.sortByTime') }}</option>
        </select>
      </div>

      <!-- 提示信息 -->
      <div class="hint-box">
        <p>{{ t('mcp.importHint') }}</p>
        <p v-if="previewMode" class="preview-warning">{{ t('mcp.previewOnly') }}</p>
      </div>

      <!-- 加载状态 -->
      <div v-if="loading" class="loading-state">
        <Loader2 :size="16" class="animate-spin" />
        <span>{{ t('mcp.refresh') }}...</span>
      </div>

      <!-- 空状态 -->
      <div v-else-if="mcps.length === 0" class="empty-state">
        <Bot :size="32" class="empty-icon" />
        <span>{{ t('mcp.empty') }}</span>
      </div>

      <!-- 无搜索结果 -->
      <div v-else-if="filteredMcps.length === 0" class="empty-state">
        <Search :size="32" class="empty-icon" />
        <span>{{ t('mcp.noSelection') }}</span>
        <button class="clear-btn" @click="clearSearch">{{ t('mcp.filterAll') }}</button>
      </div>

      <!-- 服务器列表（简化DOM结构） -->
      <div v-else class="server-list">
        <div
          v-for="item in filteredMcps"
          :key="item.name"
          class="server-card"
          :class="{ expanded: expandedServer === item.name, busy: busyServer === item.name }"
        >
          <!-- 卡片头部（可点击展开） -->
          <div class="card-header" @click="toggleExpand(item.name)">
            <div class="header-left">
              <!-- 状态指示器（SVG圆点） -->
              <div class="status-dot" :class="item.status.state"></div>

              <!-- 服务器名称 -->
              <span class="server-name">{{ item.name }}</span>

              <!-- 状态徽章 -->
              <span class="mcp-status-badge" :class="stateClass(item.status.state)">
                {{ stateLabel(item.status.state) }}
              </span>

              <!-- 传输协议标签 -->
              <span class="mcp-transport-tag">
                {{ item.transport === 'http' ? t('mcp.transportHttp') : t('mcp.transportStdio') }}
              </span>

              <!-- 工具数量徽章 -->
              <span v-if="item.status.tool_count > 0" class="mcp-tools-badge">
                <Wrench :size="10" />
                {{ item.status.tool_count }}
              </span>
            </div>

            <div class="header-right">
              <!-- 展开箭头 -->
              <ChevronRight
                :size="16"
                class="expand-chevron"
                :class="{ rotated: expandedServer === item.name }"
              />
            </div>
          </div>

          <!-- 展开的详情区域 -->
          <div v-if="expandedServer === item.name" class="card-details">
            <!-- 连接信息 -->
            <div class="detail-section">
              <div class="section-label">{{ t('mcp.connectionInfo') }}</div>
              <div class="connection-info">
                <code class="command-code">
                  {{ item.transport === 'http' ? item.url : `${item.command} ${item.args.join(' ')}` }}
                </code>
                <button
                  class="copy-btn"
                  :disabled="copiedName === item.name"
                  @click.stop="copyCommand(item)"
                >
                  <Check v-if="copiedName === item.name" :size="12" />
                  <Copy v-else :size="12" />
                </button>
              </div>
            </div>

            <!-- 元数据 -->
            <div class="metadata-row">
              <span class="metadata-item">
                {{ t('mcp.timeout') }}: {{ item.tool_timeout }}s
              </span>
              <span v-if="item.status.checked_at" class="metadata-item">
                {{ t('mcp.checkedAt') }}: {{ item.status.checked_at }}
              </span>
            </div>

            <!-- 错误信息 -->
            <div v-if="item.status.error" class="error-box">
              <CircleAlert :size="14" class="error-icon" />
              <span class="error-text">{{ item.status.error }}</span>
            </div>

            <!-- 操作按钮组 -->
            <div class="action-group">
              <button
                class="action-btn-sm"
                :disabled="previewMode || busyServer === item.name"
                @click.stop="refreshOne(item.name)"
              >
                <RefreshCcw :size="12" :class="{ 'animate-spin': busyServer === item.name }" />
                {{ t('mcp.test') }}
              </button>

              <button
                class="action-btn-sm"
                :disabled="previewMode || busyServer === item.name"
                @click.stop="openEdit(item)"
              >
                {{ t('mcp.edit') }}
              </button>

              <button
                class="action-btn-sm"
                :disabled="previewMode || busyServer === item.name"
                @click.stop="toggleMcp(item)"
              >
                <Power :size="12" />
                {{ item.enabled ? t('mcp.toggleOff') : t('mcp.toggleOn') }}
              </button>

              <button
                class="action-btn-sm action-btn-danger"
                :disabled="previewMode || busyServer === item.name"
                @click.stop="removeMcp(item.name)"
              >
                <Trash2 :size="12" />
                {{ t('mcp.delete') }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 消息提示 -->
      <p v-if="importMessage" class="success-msg">{{ importMessage }}</p>
      <p v-if="error" class="error-msg">{{ error }}</p>
    </div>

    <!-- 编辑器弹窗 -->
    <div v-if="showEditor" class="modal-overlay">
      <div class="modal-panel">
        <div class="modal-header">
          <div class="modal-title">
            <h4>{{ editorMode === 'create' ? t('mcp.formCreate') : t('mcp.formEdit') }}</h4>
            <p>{{ t('mcp.currentUsing') }}</p>
          </div>
          <button class="modal-close-btn" @click="closeEditor">
            <X :size="16" />
          </button>
        </div>

        <div class="modal-body">
          <!-- 表单区域 -->
          <div class="form-section">
            <label class="form-field">
              <span class="field-label">{{ t('mcp.name') }}</span>
              <input
                v-model="form.name"
                class="field-input"
                :placeholder="t('mcp.name')"
                @input="syncRawJsonFromForm"
              />
            </label>

            <div class="form-row">
              <label class="form-field">
                <span class="field-label">{{ t('mcp.transport') }}</span>
                <select
                  v-model="form.transport"
                  class="field-select"
                  @change="syncRawJsonFromForm"
                >
                  <option value="stdio">{{ t('mcp.transportStdio') }}</option>
                  <option value="http">{{ t('mcp.transportHttp') }}</option>
                </select>
              </label>

              <label class="form-checkbox">
                <input
                  v-model="form.enabled"
                  type="checkbox"
                  @change="syncRawJsonFromForm"
                />
                <span>{{ t('mcp.enabled') }}</span>
              </label>
            </div>

            <template v-if="form.transport === 'stdio'">
              <label class="form-field">
                <span class="field-label">{{ t('mcp.command') }}</span>
                <input
                  v-model="form.command"
                  class="field-input"
                  :placeholder="t('mcp.command')"
                  @input="syncRawJsonFromForm"
                />
              </label>

              <div class="form-field">
                <div class="field-header">
                  <span class="field-label">{{ t('mcp.args') }}</span>
                  <button class="add-btn" @click="addArg">{{ t('mcp.addArg') }}</button>
                </div>
                <div class="args-list">
                  <div v-for="(_, index) in form.args" :key="`arg-${index}`" class="arg-item">
                    <input
                      v-model="form.args[index]"
                      class="arg-input"
                      :placeholder="t('mcp.argPlaceholder')"
                      @input="syncRawJsonFromForm"
                    />
                    <button
                      class="remove-btn"
                      @click="form.args.splice(index, 1); syncRawJsonFromForm()"
                    >
                      <X :size="12" />
                    </button>
                  </div>
                </div>
                <p class="field-hint">{{ t('mcp.argsHint') }}</p>
              </div>

              <div class="form-field">
                <div class="field-header">
                  <span class="field-label">{{ t('mcp.env') }}</span>
                  <button class="add-btn" @click="addEnv">{{ t('mcp.addEnv') }}</button>
                </div>
                <div class="env-list">
                  <div v-for="(item, index) in form.env" :key="`env-${index}`" class="env-item">
                    <input
                      v-model="item.key"
                      class="env-key-input"
                      :placeholder="t('mcp.envKeyPlaceholder')"
                      @input="syncRawJsonFromForm"
                    />
                    <input
                      v-model="item.value"
                      class="env-value-input"
                      :placeholder="t('mcp.envValuePlaceholder')"
                      @input="syncRawJsonFromForm"
                    />
                    <button
                      class="remove-btn"
                      @click="form.env.splice(index, 1); syncRawJsonFromForm()"
                    >
                      <X :size="12" />
                    </button>
                  </div>
                </div>
              </div>
            </template>

            <template v-else>
              <label class="form-field">
                <span class="field-label">{{ t('mcp.url') }}</span>
                <div class="url-input-wrapper">
                  <Globe :size="14" class="url-icon" />
                  <input
                    v-model="form.url"
                    class="url-input"
                    :placeholder="t('mcp.url')"
                    @input="syncRawJsonFromForm"
                  />
                </div>
              </label>
            </template>

            <label class="form-field">
              <span class="field-label">{{ t('mcp.timeout') }}</span>
              <input
                v-model.number="form.tool_timeout"
                type="number"
                min="1"
                class="field-input"
                @input="syncRawJsonFromForm"
              />
            </label>
          </div>

          <!-- JSON编辑区域 -->
          <div class="json-section">
            <div class="json-header">
              <span class="json-label">{{ t('mcp.editJson') }}</span>
              <div class="json-actions">
                <button class="json-btn" @click="syncRawJsonFromForm">
                  {{ t('mcp.syncJson') }}
                </button>
                <button class="json-btn" @click="applyRawJsonToForm">
                  {{ t('mcp.applyJson') }}
                </button>
              </div>
            </div>
            <textarea
              v-model="rawJson"
              class="json-editor"
              spellcheck="false"
            />
          </div>
        </div>

        <div class="modal-footer">
          <button class="footer-btn footer-btn-cancel" @click="closeEditor">
            {{ t('mcp.cancel') }}
          </button>
          <button
            class="footer-btn footer-btn-save"
            :disabled="saving"
            @click="submitForm"
          >
            <Loader2 v-if="saving" :size="14" class="animate-spin" />
            {{ saving ? t('mcp.saving') : t('mcp.save') }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* 状态卡片 */
.stat-card {
  border-radius: 12px;
  padding: 14px 16px;
  border: 1px solid var(--line);
  background: var(--panel-solid);
  transition: all 0.15s ease;
}

.stat-card:hover {
  transform: translateY(-1px);
  box-shadow: var(--shadow);
}

.stat-card-total {
  border-color: rgba(107, 114, 128, 0.2);
}

.stat-card-total .stat-label {
  color: var(--text-muted);
}

.stat-card-total .stat-value {
  color: var(--text);
}

.stat-card-online {
  border-color: rgba(16, 185, 129, 0.2);
  background: linear-gradient(135deg, rgba(16, 185, 129, 0.05), transparent);
}

.stat-card-online .stat-label {
  color: #059669;
}

.stat-card-online .stat-value {
  color: #047857;
}

.stat-card-degraded {
  border-color: rgba(245, 158, 11, 0.2);
  background: linear-gradient(135deg, rgba(245, 158, 11, 0.05), transparent);
}

.stat-card-degraded .stat-label {
  color: #d97706;
}

.stat-card-degraded .stat-value {
  color: #b45309;
}

.stat-card-disabled {
  border-color: rgba(100, 116, 139, 0.2);
  background: linear-gradient(135deg, rgba(100, 116, 139, 0.05), transparent);
}

.stat-card-disabled .stat-label {
  color: #64748b;
}

.stat-card-disabled .stat-value {
  color: #475569;
}

.stat-label {
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.3px;
}

.stat-value {
  margin-top: 6px;
  font-size: 22px;
  font-weight: 700;
}

/* 主列表面板 */
.mcp-list-panel {
  border-radius: 14px;
  border: 1px solid var(--line);
  background: var(--panel-solid);
  padding: 18px;
  space-y: 16px;
}

/* 头部工具栏 */
.list-header {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 14px;
}

.header-title {
  flex: 1;
  min-width: 200px;
}

.title-text {
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}

.title-desc {
  font-size: 12px;
  color: var(--text-muted);
  margin-top: 2px;
}

.header-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

/* 操作按钮 */
.action-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  border: 1px solid var(--line);
  background: transparent;
  color: var(--text);
}

.action-btn:hover:not(:disabled) {
  background: var(--nav-hover);
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.action-btn-primary {
  background: linear-gradient(135deg, var(--brand), var(--brand-light));
  border-color: transparent;
  color: white;
  box-shadow: 0 2px 8px rgba(236, 72, 153, 0.2);
}

.action-btn-primary:hover:not(:disabled) {
  filter: brightness(1.05);
  box-shadow: 0 4px 12px rgba(236, 72, 153, 0.25);
}

.action-btn-secondary {
  background: var(--panel);
}

/* 搜索和筛选栏 */
.filter-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 12px;
}

.search-input-wrapper {
  flex: 1;
  min-width: 200px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-radius: 8px;
  border: 1px solid var(--line);
  background: var(--panel);
}

.search-icon {
  color: var(--text-muted);
  flex-shrink: 0;
}

.search-input {
  flex: 1;
  border: none;
  background: transparent;
  font-size: 12px;
  color: var(--text);
  outline: none;
}

.search-input::placeholder {
  color: var(--text-muted);
}

.clear-search-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  border-radius: 4px;
}

.clear-search-btn:hover {
  background: var(--nav-hover);
  color: var(--text);
}

.filter-select {
  padding: 8px 12px;
  border-radius: 8px;
  border: 1px solid var(--line);
  background: var(--panel);
  font-size: 12px;
  color: var(--text);
  cursor: pointer;
  min-width: 100px;
}

.filter-select:focus {
  outline: none;
  border-color: var(--brand);
}

/* 提示框 */
.hint-box {
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px dashed var(--line);
  background: rgba(249, 250, 251, 0.5);
  font-size: 11px;
  color: var(--text-muted);
  margin-bottom: 12px;
}

.preview-warning {
  margin-top: 6px;
  color: #d97706;
}

/* 加载和空状态 */
.loading-state,
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 32px;
  gap: 8px;
  color: var(--text-muted);
  font-size: 13px;
}

.empty-icon {
  opacity: 0.5;
  color: var(--text-muted);
}

.clear-btn {
  margin-top: 8px;
  padding: 6px 12px;
  border-radius: 6px;
  border: 1px solid var(--line);
  background: transparent;
  font-size: 12px;
  color: var(--text);
  cursor: pointer;
}

.clear-btn:hover {
  background: var(--nav-hover);
}

/* 服务器列表 */
.server-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

/* 服务器卡片（简化DOM结构） */
.server-card {
  border-radius: 10px;
  border: 1px solid var(--line);
  background: var(--panel);
  transition: all 0.15s ease;
  overflow: hidden;
}

.server-card:hover {
  border-color: rgba(236, 72, 153, 0.2);
}

.server-card.expanded {
  border-color: var(--brand);
  background: var(--panel-solid);
}

.server-card.busy {
  opacity: 0.7;
}

/* 卡片头部 */
.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
  cursor: pointer;
  user-select: none;
  transition: background 0.15s ease;
}

.card-header:hover {
  background: var(--nav-hover);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
}

/* 状态指示器（SVG圆点风格） */
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.status-dot.connected {
  background: #22c55e;
  box-shadow: 0 0 4px rgba(34, 197, 94, 0.4);
}

.status-dot.degraded {
  background: #f59e0b;
  box-shadow: 0 0 4px rgba(245, 158, 11, 0.4);
}

.status-dot.disabled {
  background: #94a3b8;
}

.status-dot.invalid {
  background: #ef4444;
  box-shadow: 0 0 4px rgba(239, 68, 68, 0.4);
}

.server-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  truncate: true;
}

.header-right {
  display: flex;
  align-items: center;
}

.expand-chevron {
  color: var(--text-muted);
  transition: transform 0.2s ease;
}

.expand-chevron.rotated {
  transform: rotate(90deg);
  color: var(--brand);
}

/* 展开的详情区域 */
.card-details {
  padding: 12px 14px;
  border-top: 1px solid var(--line);
  background: rgba(255, 255, 255, 0.5);
}

.detail-section {
  margin-bottom: 10px;
}

.section-label {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-muted);
  margin-bottom: 4px;
}

.connection-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

.command-code {
  flex: 1;
  padding: 6px 10px;
  border-radius: 6px;
  background: rgba(15, 23, 42, 0.05);
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 11px;
  color: var(--text);
  word-break: break-all;
}

.copy-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 6px;
  border-radius: 6px;
  border: 1px solid var(--line);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.15s ease;
}

.copy-btn:hover:not(:disabled) {
  background: var(--nav-hover);
  color: var(--text);
}

.copy-btn:disabled {
  background: rgba(34, 197, 94, 0.1);
  color: #22c55e;
  border-color: rgba(34, 197, 94, 0.2);
}

.metadata-row {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  font-size: 11px;
  color: var(--text-muted);
  margin-bottom: 8px;
}

.metadata-item {
  display: inline-flex;
  align-items: center;
}

.error-box {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  padding: 8px 10px;
  border-radius: 6px;
  background: rgba(239, 68, 68, 0.08);
  margin-bottom: 10px;
}

.error-icon {
  color: #ef4444;
  flex-shrink: 0;
  margin-top: 1px;
}

.error-text {
  font-size: 11px;
  color: #dc2626;
  word-break: break-word;
}

/* 操作按钮组 */
.action-group {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.action-btn-sm {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 6px 10px;
  border-radius: 6px;
  border: 1px solid var(--line);
  background: transparent;
  font-size: 11px;
  color: var(--text);
  cursor: pointer;
  transition: all 0.15s ease;
}

.action-btn-sm:hover:not(:disabled) {
  background: var(--nav-hover);
}

.action-btn-sm:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.action-btn-danger {
  color: #dc2626;
  border-color: rgba(239, 68, 68, 0.2);
}

.action-btn-danger:hover:not(:disabled) {
  background: rgba(239, 68, 68, 0.08);
}

/* 消息提示 */
.success-msg {
  font-size: 12px;
  color: #059669;
  margin-top: 8px;
}

.error-msg {
  font-size: 12px;
  color: #dc2626;
  word-break: break-word;
  margin-top: 8px;
}

/* 编辑器弹窗 */
.modal-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.4);
  backdrop-filter: blur(4px);
}

.modal-panel {
  width: 100%;
  max-width: 900px;
  max-height: 90vh;
  border-radius: 16px;
  border: 1px solid var(--line);
  background: var(--panel-solid);
  box-shadow: var(--shadow);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.modal-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--line);
}

.modal-title h4 {
  font-size: 16px;
  font-weight: 600;
  color: var(--text);
}

.modal-title p {
  font-size: 12px;
  color: var(--text-muted);
  margin-top: 2px;
}

.modal-close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 8px;
  border-radius: 8px;
  border: 1px solid var(--line);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}

.modal-close-btn:hover {
  background: var(--nav-hover);
  color: var(--text);
}

.modal-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
}

@media (max-width: 768px) {
  .modal-body {
    grid-template-columns: 1fr;
  }
}

/* 表单样式 */
.form-section {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-muted);
}

.field-input,
.field-select {
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid var(--line);
  background: var(--panel);
  font-size: 13px;
  color: var(--text);
}

.field-input:focus,
.field-select:focus {
  outline: none;
  border-color: var(--brand);
}

.field-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.add-btn {
  font-size: 12px;
  color: var(--brand);
  background: transparent;
  border: none;
  cursor: pointer;
}

.add-btn:hover {
  color: var(--brand-light);
}

.args-list,
.env-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.arg-item,
.env-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.arg-input,
.env-key-input,
.env-value-input {
  flex: 1;
  padding: 8px 10px;
  border-radius: 6px;
  border: 1px solid var(--line);
  background: var(--panel);
  font-size: 12px;
  color: var(--text);
}

.arg-input:focus,
.env-key-input:focus,
.env-value-input:focus {
  outline: none;
  border-color: var(--brand);
}

.env-item {
  grid-template-columns: 1fr 1fr auto;
  display: grid;
}

.remove-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px;
  border-radius: 4px;
  border: none;
  background: transparent;
  color: #dc2626;
  cursor: pointer;
}

.remove-btn:hover {
  background: rgba(239, 68, 68, 0.1);
}

.field-hint {
  font-size: 11px;
  color: var(--text-muted);
  margin-top: 2px;
}

.form-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.form-checkbox {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-top: 24px;
  font-size: 13px;
  color: var(--text);
  cursor: pointer;
}

.form-checkbox input {
  width: 16px;
  height: 16px;
  accent-color: var(--brand);
}

.url-input-wrapper {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid var(--line);
  background: var(--panel);
}

.url-icon {
  color: var(--text-muted);
  flex-shrink: 0;
}

.url-input {
  flex: 1;
  border: none;
  background: transparent;
  font-size: 13px;
  color: var(--text);
}

.url-input:focus {
  outline: none;
}

/* JSON编辑区域 */
.json-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.json-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.json-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-muted);
}

.json-actions {
  display: flex;
  gap: 6px;
}

.json-btn {
  padding: 6px 10px;
  border-radius: 6px;
  border: 1px solid var(--line);
  background: transparent;
  font-size: 11px;
  color: var(--text);
  cursor: pointer;
}

.json-btn:hover {
  background: var(--nav-hover);
}

.json-editor {
  flex: 1;
  min-height: 400px;
  padding: 14px;
  border-radius: 8px;
  border: 1px solid var(--line);
  background: #0f172a;
  font-family: 'SF Mono', 'Fira Code', monospace;
  font-size: 11px;
  color: #e2e8f0;
  resize: none;
}

.json-editor:focus {
  outline: none;
  border-color: var(--brand);
}

/* 弹窗底部 */
.modal-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  padding: 16px 20px;
  border-top: 1px solid var(--line);
}

.footer-btn {
  padding: 10px 16px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.footer-btn-cancel {
  border: 1px solid var(--line);
  background: transparent;
  color: var(--text);
}

.footer-btn-cancel:hover {
  background: var(--nav-hover);
}

.footer-btn-save {
  background: linear-gradient(135deg, var(--brand), var(--brand-light));
  border: none;
  color: white;
  box-shadow: 0 2px 8px rgba(236, 72, 153, 0.2);
}

.footer-btn-save:hover:not(:disabled) {
  filter: brightness(1.05);
  box-shadow: 0 4px 12px rgba(236, 72, 153, 0.25);
}

.footer-btn-save:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>