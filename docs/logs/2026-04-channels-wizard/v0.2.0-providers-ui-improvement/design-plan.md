# 供应商管理页面 UI 改进方案

**版本**: v0.2.0  
**日期**: 2026-04-04  
**范围**: 仅 UI 改进，不涉及后端逻辑

---

## 1. 原型图分析

### 1.1 原型图展示的状态

**空状态（Empty State）**:
- 居中显示提示文字："暂无通道！点击创建通道或是向导连接至聊天软件"
- 无供应商卡片
- 引导用户进行首次配置

**激活状态（Active State）**:
- 供应商卡片网格布局
- 每张卡片包含：
  - 供应商图标（如"飞书"logo）
  - 供应商名称
  - 状态标签："已激活"（绿色）
  - 操作按钮：删除、编辑

### 1.2 与当前实现的差距

**当前 ProvidersSettings.vue 实现**:
- 双栏布局：左侧供应商列表 + 右侧详细配置表单
- 复杂的模型管理功能（模型选择网格、测试连接、保存模型）
- 技术导向的 UI，适合高级用户
- 1200+ 行代码，逻辑复杂

**原型图设计理念**:
- 卡片式布局，直观展示所有供应商
- 简化的操作流程
- 注重视觉反馈和状态展示
- 适合快速切换和管理

### 1.3 核心功能模块识别

需要实现的 UI 组件：
1. **ProvidersCardView** - 供应商卡片网格视图
2. **ProviderCard** - 单个供应商卡片组件
3. **ProviderWizardModal** - 配置向导模态框（前端 UI-only）
4. **EmptyState** - 空状态引导组件
5. **StatusBadge** - 状态徽章组件

---

## 2. OpenAkita 参考研究总结

### 2.1 QR 码配置实现机制（参考）

**OpenAkita 模式**:
- 使用独立 Modal 组件显示 QR 码
- 状态机：`idle` → `loading` → `scanning` → `polling` → `success/error`
- 平台特定 Modal：WechatQRModal, FeishuQRModal, QQBotQRModal, WecomQRModal

**Agent Diva GUI 适配方案（UI-only）**:
- 使用 Vue 3 Composition API 实现类似状态机
- 创建 `ProviderQRModal.vue` 组件（仅展示 QR 码图片）
- 状态管理：`useState` → `useRef` + `reactive`

### 2.2 用户引导向导设计

**OpenAkita 向导流程**:
```
ob-welcome → ob-agreement → ob-llm → ob-im → ob-cli → ob-progress → ob-done
```

**配置向导步骤**（简化版）:
```
step-1: 选择供应商类型 → step-2: 输入 API Key → step-3: 配置 Base URL → step-4: 测试连接 → done
```

**Agent Diva GUI 实现**:
- 使用 Teleport 到 body 的模态框
- CSS transitions (fade + slide)
- Props/emit 模式与父组件通信
- 步骤状态管理：`currentStep` ref

### 2.3 前端架构模式

**React/TypeScript 模式**:
- Hooks: `useState`, `useEffect`, `useCallback`
- Context API 进行状态共享
- shadcn/ui 组件库

**Vue 3 + TypeScript 适配**:
- Composition API: `ref`, `reactive`, `computed`, `watch`
- Provide/Inject 模式
- 使用 lucide-vue-next 图标库
- Tailwind CSS + 自定义 CSS 变量系统

### 2.4 配置管理与 UI 组件组织

**OpenAkita 模式**:
- .env 文件存储配置
- HTTP API + Tauri invoke 双模式
- useEnvManager Hook 统一管理

**Agent Diva GUI 现有模式**:
- Tauri invoke 调用 Rust 后端
- 本地状态 refs 管理临时配置
- Props 从父组件接收配置数据

---

## 3. UI 布局与交互设计

### 3.1 主界面布局结构

```
┌─────────────────────────────────────────────┐
│  设置标题栏                                  │
├─────────────────────────────────────────────┤
│                                             │
│  ┌─────────────────────────────────────┐   │
│  │  + 添加供应商  [导入配置]            │   │
│  └─────────────────────────────────────┘   │
│                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐ │
│  │ 飞书图标 │  │ DeepSeek │  │ OpenAI   │ │
│  │   飞书   │  │  DeepSeek│  │  OpenAI  │ │
│  │ [已激活] │  │  [就绪]  │  │ [未配置] │ │
│  │ [编辑]   │  │  [测试]  │  │  [配置]  │ │
│  └──────────┘  └──────────┘  └──────────┘ │
│                                             │
│  （空状态时显示引导提示）                    │
│  "暂无供应商！点击"添加供应商"或跟随向导配置"   │
│                                             │
└─────────────────────────────────────────────┘
```

### 3.2 组件设计和状态管理

#### ProviderCard 组件

```typescript
interface ProviderCardProps {
  name: string;
  displayName: string;
  icon: string;
  status: 'ready' | 'missingConfig' | 'active';
  currentModel?: string;
}

// 内部状态
const isHovered = ref(false);
const isTesting = ref(false);
const testResult = ref<'idle' | 'success' | 'failed'>('idle');
```

#### ProvidersCardView 组件

```typescript
interface ProviderCardItem {
  id: string;
  name: string;
  displayName: string;
  icon: string | Component;
  status: 'ready' | 'missingConfig' | 'active';
  currentModel?: string;
  apiBase?: string;
}

// Props
const props = defineProps<{
  providers: ProviderCardItem[];
  isLoading: boolean;
}>();

// Emits
const emit = defineEmits<{
  (e: 'edit', provider: ProviderCardItem): void;
  (e: 'delete', provider: ProviderCardItem): void;
  (e: 'test', provider: ProviderCardItem): void;
  (e: 'create'): void;
}>();
```

#### ProviderWizardModal 组件

```typescript
type WizardStep = 'select-provider' | 'api-key' | 'api-base' | 'test' | 'done';

interface WizardState {
  currentStep: WizardStep;
  selectedProvider: string | null;
  apiKey: string;
  apiBase: string;
  testStatus: 'idle' | 'testing' | 'success' | 'failed';
  testMessage: string;
}

// Props
const props = defineProps<{
  open: boolean;
  providers: ProviderSpec[];
}>();

// Emits
const emit = defineEmits<{
  (e: 'update:open', value: boolean): void;
  (e: 'complete', config: ProviderConfig): void;
}>();
```

### 3.3 用户交互流程

**创建供应商流程**:
1. 点击 "添加供应商" 按钮
2. 弹出向导模态框
3. Step 1: 从预设列表选择供应商类型（DeepSeek, OpenAI, etc.）
4. Step 2: 输入 API Key（带显示/隐藏切换）
5. Step 3: 配置 API Base URL（可选，有默认值）
6. Step 4: 点击"测试连接"按钮，显示测试结果
7. 完成：保存配置并关闭模态框

**编辑供应商流程**:
1. 点击卡片上的"编辑"按钮
2. 弹出向导模态框（预填充当前配置）
3. 修改配置后保存

**测试连接流程**:
1. 点击卡片上的"测试"按钮
2. 显示加载状态和 spinner
3. 显示测试结果（成功：绿色；失败：红色 + 错误信息）
4. 3 秒后自动隐藏结果

---

## 4. 技术实现方案

### 4.1 文件清单

**新增文件**:
```
agent-diva-gui/src/components/settings/
├── ProvidersCardView.vue          # 卡片网格视图主组件
├── ProviderCard.vue               # 单个供应商卡片
├── ProviderWizardModal.vue        # 配置向导模态框
└── EmptyState.vue                 # 空状态引导组件

agent-diva-gui/src/styles/
└── providers-card.css             # 卡片视图专用样式（可选，可并入 styles.css）
```

**修改文件**:
```
agent-diva-gui/src/components/SettingsView.vue      # 添加卡片视图切换逻辑
agent-diva-gui/src/locales/zh.ts                    # 添加新的 i18n 键
agent-diva-gui/src/locales/en.ts                    # 添加英文翻译
agent-diva-gui/src/styles.css                       # 添加卡片样式
```

### 4.2 关键代码示例

#### ProviderCard.vue

```vue
<script setup lang="ts">
import { ref } from 'vue';
import { Server, Check, LoaderCircle, CircleAlert, PlugZap, Trash2, Edit3 } from 'lucide-vue-next';

const props = defineProps<{
  name: string;
  displayName: string;
  icon?: string;
  status: 'ready' | 'missingConfig' | 'active';
  currentModel?: string;
}>();

const emit = defineEmits<{
  (e: 'edit'): void;
  (e: 'delete'): void;
  (e: 'test'): void;
}>();

const isHovered = ref(false);
const isTesting = ref(false);
const testStatus = ref<'idle' | 'success' | 'failed'>('idle');

const statusMap = {
  ready: { label: '就绪', class: 'text-success' },
  missingConfig: { label: '需配置', class: 'text-warning' },
  active: { label: '已激活', class: 'text-success' },
};
</script>

<template>
  <div 
    class="providers-card"
    :class="{ 'is-active': status === 'active' }"
    @mouseenter="isHovered = true"
    @mouseleave="isHovered = false"
  >
    <div class="providers-card-icon">
      <Server :size="24" />
    </div>
    
    <div class="providers-card-body">
      <h3 class="providers-card-title">{{ displayName }}</h3>
      <div class="providers-card-status" :class="statusMap[status].class">
        <Check v-if="status === 'ready' || status === 'active'" :size="12" />
        <span>{{ statusMap[status].label }}</span>
      </div>
      <p v-if="currentModel" class="providers-card-model">
        {{ currentModel }}
      </p>
    </div>
    
    <div class="providers-card-actions" :class="{ visible: isHovered || isTesting }">
      <button 
        class="providers-card-action-btn"
        @click.stop="emit('test')"
        :disabled="isTesting"
      >
        <LoaderCircle v-if="isTesting" :size="14" class="animate-spin" />
        <PlugZap v-else :size="14" />
      </button>
      <button 
        class="providers-card-action-btn"
        @click.stop="emit('edit')"
      >
        <Edit3 :size="14" />
      </button>
      <button 
        class="providers-card-action-btn providers-card-action-btn-danger"
        @click.stop="emit('delete')"
      >
        <Trash2 :size="14" />
      </button>
    </div>
  </div>
</template>
```

#### ProviderWizardModal.vue

```vue
<script setup lang="ts">
import { ref, computed } from 'vue';
import { X, ChevronRight, LoaderCircle, Check, CircleAlert } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const props = defineProps<{
  open: boolean;
  providers: any[];
  initialData?: {
    name?: string;
    apiKey?: string;
    apiBase?: string;
  };
}>();

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void;
  (e: 'complete', config: any): void;
}>();

type Step = 'select' | 'apikey' | 'apibase' | 'test' | 'done';
const currentStep = ref<Step>('select');

const formData = ref({
  selectedProvider: props.initialData?.name || '',
  apiKey: props.initialData?.apiKey || '',
  apiBase: props.initialData?.apiBase || '',
});

const isTesting = ref(false);
const testResult = ref<'idle' | 'success' | 'failed'>('idle');
const testMessage = ref('');

const steps = [
  { key: 'select', title: '选择供应商' },
  { key: 'apikey', title: 'API Key' },
  { key: 'apibase', title: 'API Base' },
  { key: 'test', title: '测试连接' },
];

const currentStepIndex = computed(() => 
  steps.findIndex(s => s.key === currentStep.value)
);

const canNext = computed(() => {
  if (currentStep.value === 'select') return formData.value.selectedProvider;
  if (currentStep.value === 'apikey') return formData.value.apiKey.length > 0;
  if (currentStep.value === 'apibase') return true; // 有默认值
  if (currentStep.value === 'test') return testResult.value === 'success';
  return false;
});

const nextStep = () => {
  const currentIndex = currentStepIndex.value;
  if (currentIndex < steps.length - 1) {
    currentStep.value = steps[currentIndex + 1].key as Step;
  }
};

const prevStep = () => {
  const currentIndex = currentStepIndex.value;
  if (currentIndex > 0) {
    currentStep.value = steps[currentIndex - 1].key as Step;
  }
};

const testConnection = async () => {
  isTesting.value = true;
  testResult.value = 'idle';
  
  // TODO: 实际调用时通过 emit 通知父组件执行测试
  // 这里仅展示 UI 状态
  setTimeout(() => {
    isTesting.value = false;
    testResult.value = 'success'; // 或 'failed'
    testMessage.value = '连接成功！';
  }, 1500);
};

const complete = () => {
  emit('complete', {
    name: formData.value.selectedProvider,
    apiKey: formData.value.apiKey,
    apiBase: formData.value.apiBase,
  });
  emit('update:open', false);
};

const close = () => {
  emit('update:open', false);
  // 重置状态
  currentStep.value = 'select';
  testResult.value = 'idle';
};
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="open" class="wizard-overlay" @click.self="close">
        <div class="wizard-modal">
          <!-- Header -->
          <div class="wizard-header">
            <h3 class="wizard-title">{{ t('providers.wizardTitle') }}</h3>
            <button class="wizard-close" @click="close">
              <X :size="18" />
            </button>
          </div>
          
          <!-- Progress Steps -->
          <div class="wizard-progress">
            <div 
              v-for="(step, index) in steps" 
              :key="step.key"
              class="wizard-progress-item"
              :class="{ 
                active: step.key === currentStep,
                completed: index < currentStepIndex 
              }"
            >
              <div class="wizard-progress-indicator">
                <Check v-if="index < currentStepIndex" :size="12" />
                <span v-else>{{ index + 1 }}</span>
              </div>
              <span class="wizard-progress-label">{{ step.title }}</span>
            </div>
          </div>
          
          <!-- Step Content -->
          <div class="wizard-content">
            <!-- Step 1: Select Provider -->
            <div v-if="currentStep === 'select'" class="wizard-step">
              <label class="wizard-label">{{ t('providers.selectProviderType') }}</label>
              <select 
                v-model="formData.selectedProvider"
                class="wizard-select"
              >
                <option value="" disabled>{{ t('providers.chooseProvider') }}</option>
                <option 
                  v-for="provider in providers" 
                  :key="provider.name"
                  :value="provider.name"
                >
                  {{ provider.display_name }}
                </option>
              </select>
            </div>
            
            <!-- Step 2: API Key -->
            <div v-if="currentStep === 'apikey'" class="wizard-step">
              <label class="wizard-label">{{ t('providers.apiKey') }}</label>
              <input
                v-model="formData.apiKey"
                type="password"
                :placeholder="t('providers.enterApiKey')"
                class="wizard-input"
                autocomplete="off"
              />
            </div>
            
            <!-- Step 3: API Base -->
            <div v-if="currentStep === 'apibase'" class="wizard-step">
              <label class="wizard-label">{{ t('providers.apiBaseUrl') }}</label>
              <input
                v-model="formData.apiBase"
                type="text"
                :placeholder="t('providers.placeholderLocalCustom')"
                class="wizard-input"
              />
              <p class="wizard-hint">{{ t('providers.apiBaseHint') }}</p>
            </div>
            
            <!-- Step 4: Test -->
            <div v-if="currentStep === 'test'" class="wizard-step">
              <label class="wizard-label">{{ t('providers.testConnection') }}</label>
              <div class="wizard-test-area">
                <button
                  class="wizard-test-btn"
                  @click="testConnection"
                  :disabled="isTesting"
                >
                  <LoaderCircle v-if="isTesting" :size="16" class="animate-spin" />
                  <PlugZap v-else :size="16" />
                  <span>{{ isTesting ? t('providers.testingConnection') : t('providers.testConnection') }}</span>
                </button>
                
                <div v-if="testResult === 'success'" class="wizard-test-result success">
                  <Check :size="16" />
                  <span>{{ testMessage }}</span>
                </div>
                <div v-if="testResult === 'failed'" class="wizard-test-result failed">
                  <CircleAlert :size="16" />
                  <span>{{ testMessage }}</span>
                </div>
              </div>
            </div>
            
            <!-- Step 5: Done -->
            <div v-if="currentStep === 'done'" class="wizard-step">
              <div class="wizard-done">
                <Check :size="48" class="wizard-done-icon" />
                <h4>{{ t('providers.wizardDone') }}</h4>
                <p>{{ t('providers.wizardDoneHint') }}</p>
              </div>
            </div>
          </div>
          
          <!-- Footer Actions -->
          <div class="wizard-footer">
            <button
              v-if="currentStepIndex > 0 && currentStep !== 'done'"
              class="wizard-btn wizard-btn-secondary"
              @click="prevStep"
            >
              {{ t('mcp.back') }}
            </button>
            
            <button
              v-if="currentStep !== 'done'"
              class="wizard-btn wizard-btn-primary"
              :disabled="!canNext"
              @click="nextStep"
            >
              {{ t('mcp.next') }}
              <ChevronRight :size="16" />
            </button>
            
            <button
              v-if="currentStep === 'done'"
              class="wizard-btn wizard-btn-primary"
              @click="complete"
            >
              {{ t('mcp.finish') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
```

### 4.3 CSS 样式（styles.css 追加）

```css
/* ========================================
   供应商卡片视图样式
   ======================================== */

/* 卡片网格容器 */
.providers-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 1.5rem;
  padding: 1.5rem 0;
}

/* 单个卡片 */
.providers-card {
  position: relative;
  background: var(--panel-solid);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  padding: 1.5rem;
  transition: all 0.15s ease;
  cursor: pointer;
  overflow: hidden;
}

.providers-card:hover {
  border-color: var(--accent-border);
  box-shadow: 0 4px 12px var(--accent-glow);
  transform: translateY(-2px);
}

.providers-card.is-active {
  border-color: var(--accent);
  box-shadow: 0 4px 16px var(--accent-glow);
}

/* 卡片图标 */
.providers-card-icon {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-sm);
  background: var(--accent-bg-light);
  color: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 1rem;
  transition: transform 0.15s ease;
}

.providers-card:hover .providers-card-icon {
  transform: scale(1.1);
}

/* 卡片主体 */
.providers-card-body {
  margin-bottom: 1rem;
}

.providers-card-title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 0.5rem;
}

.providers-card-status {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.75rem;
  font-weight: 500;
  margin-bottom: 0.25rem;
}

.providers-card-model {
  font-size: 0.75rem;
  color: var(--text-muted);
  font-family: monospace;
}

/* 卡片操作按钮 */
.providers-card-actions {
  position: absolute;
  top: 1rem;
  right: 1rem;
  display: flex;
  gap: 0.5rem;
  opacity: 0;
  transition: opacity 0.15s ease;
}

.providers-card-actions.visible {
  opacity: 1;
}

.providers-card-action-btn {
  width: 28px;
  height: 28px;
  border-radius: 6px;
  border: none;
  background: var(--panel);
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.providers-card-action-btn:hover {
  background: var(--accent-bg-light);
  color: var(--accent);
}

.providers-card-action-btn-danger:hover {
  background: var(--danger-bg);
  color: var(--danger);
}

/* 空状态 */
.providers-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 4rem 2rem;
  text-align: center;
}

.providers-empty-icon {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  background: var(--accent-bg-light);
  color: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 1.5rem;
  opacity: 0.5;
}

.providers-empty-title {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 0.5rem;
}

.providers-empty-desc {
  font-size: 0.875rem;
  color: var(--text-muted);
  max-width: 400px;
}

/* 向导模态框 */
.wizard-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.wizard-modal {
  background: var(--panel-solid);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  width: 100%;
  max-width: 560px;
  max-height: 90vh;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.wizard-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid var(--line);
}

.wizard-title {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--text);
}

.wizard-close {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.wizard-close:hover {
  background: var(--accent-bg-light);
  color: var(--accent);
}

/* 进度步骤 */
.wizard-progress {
  display: flex;
  gap: 1rem;
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid var(--line);
}

.wizard-progress-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  flex: 1;
}

.wizard-progress-indicator {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: var(--panel);
  border: 2px solid var(--line);
  color: var(--text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.75rem;
  font-weight: 600;
  transition: all 0.15s ease;
}

.wizard-progress-item.active .wizard-progress-indicator {
  border-color: var(--accent);
  color: var(--accent);
}

.wizard-progress-item.completed .wizard-progress-indicator {
  background: var(--accent);
  border-color: var(--accent);
  color: white;
}

.wizard-progress-label {
  font-size: 0.625rem;
  color: var(--text-muted);
  text-align: center;
}

/* 内容区域 */
.wizard-content {
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem;
}

.wizard-step {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.wizard-label {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
}

.wizard-input,
.wizard-select {
  width: 100%;
  padding: 0.75rem 1rem;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel);
  color: var(--text);
  font-size: 0.875rem;
  transition: all 0.15s ease;
}

.wizard-input:focus,
.wizard-select:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-glow);
}

.wizard-hint {
  font-size: 0.75rem;
  color: var(--text-muted);
  margin-top: 0.25rem;
}

.wizard-test-area {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.wizard-test-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.75rem 1.5rem;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel);
  color: var(--text);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.wizard-test-btn:hover:not(:disabled) {
  background: var(--accent-bg-light);
  border-color: var(--accent);
  color: var(--accent);
}

.wizard-test-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.wizard-test-result {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  font-weight: 500;
}

.wizard-test-result.success {
  background: var(--success);
  color: white;
}

.wizard-test-result.failed {
  background: var(--danger);
  color: white;
}

.wizard-done {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 2rem 0;
}

.wizard-done-icon {
  width: 80px;
  height: 80px;
  border-radius: 50%;
  background: var(--success);
  color: white;
  margin-bottom: 1.5rem;
}

.wizard-done h4 {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 0.5rem;
}

.wizard-done p {
  font-size: 0.875rem;
  color: var(--text-muted);
}

/* 底部按钮 */
.wizard-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 1.25rem 1.5rem;
  border-top: 1px solid var(--line);
}

.wizard-btn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.625rem 1.25rem;
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.wizard-btn-primary {
  background: var(--accent);
  color: white;
  border: none;
}

.wizard-btn-primary:hover:not(:disabled) {
  filter: brightness(1.1);
}

.wizard-btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.wizard-btn-secondary {
  background: var(--panel);
  color: var(--text);
  border: 1px solid var(--line);
}

.wizard-btn-secondary:hover {
  background: var(--accent-bg-light);
}

/* Modal 过渡动画 */
.modal-enter-active,
.modal-leave-active {
  transition: all 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
  transform: scale(0.95);
}

.modal-enter-from .wizard-modal,
.modal-leave-to .wizard-modal {
  transform: translateY(-20px);
}
```

### 4.4 i18n 新增键值

**zh.ts**:
```typescript
providers: {
  // ... 现有键 ...
  wizardTitle: '配置供应商向导',
  selectProviderType: '选择供应商类型',
  chooseProvider: '请选择供应商...',
  apiBaseHint: '如不填写将使用供应商默认 API 地址',
  wizardDone: '配置完成！',
  wizardDoneHint: '您现在可以开始使用此供应商了',
  back: '上一步',
  next: '下一步',
  finish: '完成',
}
```

---

## 5. 实施步骤

### Phase 1: 基础组件开发（1-2 小时）
1. 创建 `ProviderCard.vue` 组件
2. 创建 `ProvidersCardView.vue` 组件
3. 创建 `EmptyState.vue` 组件
4. 在 `styles.css` 中添加卡片样式

### Phase 2: 向导模态框开发（2-3 小时）
1. 创建 `ProviderWizardModal.vue` 组件
2. 实现步骤状态机逻辑
3. 实现测试连接 UI 状态
4. 添加过渡动画

### Phase 3: 集成与测试（1 小时）
1. 在 `ProvidersSettings.vue` 中添加卡片视图切换
2. 或在 `SettingsView.vue` 中添加路由到卡片视图
3. 更新 i18n 翻译文件
4. 手动测试交互流程

### Phase 4: 优化与调整（可选）
1. 根据用户反馈调整样式
2. 添加更多动画细节
3. 优化响应式布局

---

## 6. 风险与注意事项

### 6.1 技术风险
- **Tauri 集成**: 组件需要通过 emit 与父组件通信，由父组件调用 Tauri invoke
- **状态同步**: 卡片视图的状态需要与现有 ProvidersSettings 状态保持同步
- **图标资源**: 供应商图标需要使用 lucide-vue-next 或自定义 SVG

### 6.2 UI/UX 注意事项
- **主题兼容**: 确保卡片样式在 love/dark/default 三种主题下都正常显示
- **响应式**: 卡片网格应自适应窗口大小，在小屏幕上显示 1 列，大屏幕显示多列
- **无障碍**: 按钮需要适当的 aria-label 和 title 属性

### 6.3 向后兼容性
- 保留现有 `ProvidersSettings.vue` 作为高级配置入口
- 卡片视图作为默认视图，提供"高级配置"按钮切换到详细视图
- 不删除任何现有功能，仅添加新的 UI 层

---

## 7. 下一步行动

1. **确认设计方案**: 与用户确认此 UI 改进方案是否符合原型图设计理念
2. **开始实施**: 按照 Phase 1-3 的顺序逐步实现组件
3. **测试验证**: 在三种主题下测试 UI 表现，确保视觉效果一致
4. **用户验收**: 展示实现效果，收集反馈并进行调整

---

## 8. 附录：ASCII 原型图参考

```
空状态:
┌─────────────────────────────────────┐
│                                     │
│           📦                        │
│   暂无供应商！                      │
│   点击"添加供应商"或跟随向导配置    │
│                                     │
│        [添加供应商] [配置向导]      │
│                                     │
└─────────────────────────────────────┘

激活状态（卡片网格）:
┌─────────────────────────────────────┐
│  + 添加供应商  [导入配置]           │
│                                     │
│  ┌────────┐  ┌────────┐  ┌────────┐│
│  │  📱   │  │  🧠   │  │  🌐   ││
│  │  飞书  │  │DeepSeek│  │OpenAI  ││
│  │ [✓激活]│  │ [✓就绪]│  │ [⚠配置]││
│  │ [✎] [🗑]│ │ [✎] [🗑]│ │ [⚙配置]││
│  └────────┘  └────────┘  └────────┘│
│                                     │
└─────────────────────────────────────┘
```
