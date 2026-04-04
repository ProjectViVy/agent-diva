# 供应商管理卡片视图 - 使用说明

**版本**: v0.2.0  
**日期**: 2026-04-04  
**类型**: UI-only 改进（不涉及后端逻辑）

---

## 概述

本组件集为 Agent Diva GUI 的供应商管理页面提供了简化的卡片视图 UI，包括:

1. **ProviderCard.vue** - 单个供应商卡片组件
2. **ProvidersCardView.vue** - 卡片网格视图主组件
3. **ProviderWizardModal.vue** - 配置向导模态框

---

## 组件文件位置

```
agent-diva-gui/src/components/settings/
├── ProviderCard.vue           # 供应商卡片组件
├── ProvidersCardView.vue      # 卡片视图容器
└── ProviderWizardModal.vue    # 配置向导模态框
```

---

## 快速开始

### 1. 在父组件中引入

```vue
<script setup lang="ts">
import { ref } from 'vue';
import ProvidersCardView from './settings/ProvidersCardView.vue';
import ProviderWizardModal from './settings/ProviderWizardModal.vue';

interface ProviderCardItem {
  id: string;
  name: string;
  displayName: string;
  status: 'ready' | 'missingConfig' | 'active';
  currentModel?: string;
  apiBase?: string;
  isCustom?: boolean;
}

const providers = ref<ProviderCardItem[]>([
  {
    id: 'deepseek',
    name: 'deepseek',
    displayName: 'DeepSeek',
    status: 'ready',
    currentModel: 'deepseek-chat',
    apiBase: 'https://api.deepseek.com/v1',
  },
]);

const isLoading = ref(false);
const activeProviderName = ref('deepseek');
const isWizardOpen = ref(false);

const handleEdit = (provider: ProviderCardItem) => {
  console.log('Edit provider:', provider);
  // TODO: 打开向导进行编辑
};

const handleDelete = (provider: ProviderCardItem) => {
  console.log('Delete provider:', provider);
  // TODO: 实现删除逻辑
};

const handleTest = (provider: ProviderCardItem) => {
  console.log('Test provider:', provider);
  // TODO: 实现测试逻辑
};

const handleCreate = () => {
  isWizardOpen.value = true;
};

const handleWizardComplete = (data: any) => {
  console.log('Wizard completed:', data);
  // TODO: 保存配置到后端
};
</script>

<template>
  <div class="providers-settings-container">
    <ProvidersCardView
      :providers="providers"
      :is-loading="isLoading"
      :active-provider-name="activeProviderName"
      @edit="handleEdit"
      @delete="handleDelete"
      @test="handleTest"
      @create="handleCreate"
      @import="handleImport"
    />
    
    <ProviderWizardModal
      v-model:open="isWizardOpen"
      :providers="availableProviders"
      @test="handleWizardTest"
      @complete="handleWizardComplete"
    />
  </div>
</template>
```

### 2. 事件处理说明

#### ProvidersCardView 事件

| 事件名 | 参数 | 说明 |
|--------|------|------|
| `edit` | `provider: ProviderCardItem` | 用户点击编辑按钮 |
| `delete` | `provider: ProviderCardItem` | 用户点击删除按钮 |
| `test` | `provider: ProviderCardItem` | 用户点击测试按钮 |
| `create` | 无 | 用户点击添加供应商按钮 |
| `import` | 无 | 用户点击导入配置按钮 |

#### ProviderWizardModal 事件

| 事件名 | 参数 | 说明 |
|--------|------|------|
| `update:open` | `value: boolean` | 模态框打开/关闭状态 |
| `test` | `data: { selectedProvider, apiKey, apiBase }` | 测试连接请求 |
| `complete` | `data: { selectedProvider, apiKey, apiBase }` | 向导完成，提交配置 |

---

## Wizard 测试事件处理示例

```typescript
const handleWizardTest = async (data: {
  selectedProvider: string;
  apiKey: string;
  apiBase: string;
}): Promise<{ success: boolean; message: string; latency?: number }> => {
  try {
    // 调用后端 API 进行测试
    const result = await invoke('test_provider_connection', {
      provider: data.selectedProvider,
      apiKey: data.apiKey,
      apiBase: data.apiBase,
    });
    
    return {
      success: result.success,
      message: result.message,
      latency: result.latency_ms,
    };
  } catch (error) {
    return {
      success: false,
      message: error instanceof Error ? error.message : '测试失败',
    };
  }
};
```

---

## 样式集成

所有必需的 CSS 样式已添加到 `styles.css` 文件末尾，包括:

- 卡片网格布局 (`.providers-grid`)
- 单个卡片样式 (`.providers-card`)
- 空状态样式 (`.providers-empty-state`)
- 加载状态样式 (`.providers-loading`)
- 向导模态框样式 (`.wizard-overlay`, `.wizard-modal`)
- 进度步骤样式 (`.wizard-progress`)
- 按钮和输入框样式

样式使用 CSS 变量系统，自动支持三种主题:
- Love (粉色主题)
- Dark (深色主题)
- Default (简约粉白)

---

## i18n 国际化

已添加的翻译键 (zh.ts 和 en.ts):

### 中文 (zh.ts)
```typescript
providers: {
  // 卡片视图
  edit: '编辑',
  loading: '加载中...',
  emptyTitle: '暂无供应商!',
  emptyDesc: '点击"添加供应商"或跟随向导配置',
  importConfig: '导入配置',
  apiBaseHint: '如不填写将使用供应商默认 API 地址',
  // 向导
  wizardTitle: '配置供应商向导',
  wizardStepSelect: '选择供应商',
  wizardStepApiKey: 'API Key',
  wizardStepApiBase: 'API Base',
  wizardStepTest: '测试连接',
  wizardDone: '配置完成!',
  wizardDoneHint: '您现在可以开始使用此供应商了',
  wizardBack: '上一步',
  wizardNext: '下一步',
  wizardFinish: '完成',
  selectProviderType: '选择供应商类型',
  chooseProvider: '请选择供应商...',
  showApiKey: '显示 API Key',
  hideApiKey: '隐藏 API Key',
}
```

### 英文 (en.ts)
```typescript
providers: {
  // Card view
  edit: 'Edit',
  loading: 'Loading...',
  emptyTitle: 'No Providers!',
  emptyDesc: 'Click "Add Provider" or follow the wizard to configure',
  importConfig: 'Import Config',
  apiBaseHint: 'If left blank, the provider default API address will be used',
  // Wizard
  wizardTitle: 'Configure Provider Wizard',
  wizardStepSelect: 'Select Provider',
  wizardStepApiKey: 'API Key',
  wizardStepApiBase: 'API Base',
  wizardStepTest: 'Test Connection',
  wizardDone: 'Configuration Complete!',
  wizardDoneHint: 'You can now start using this provider',
  wizardBack: 'Back',
  wizardNext: 'Next',
  wizardFinish: 'Finish',
  selectProviderType: 'Select Provider Type',
  chooseProvider: 'Please select a provider...',
}
```

---

## 与现有 ProvidersSettings.vue 的关系

**重要**: 本卡片视图是作为现有 `ProvidersSettings.vue` 的**替代 UI**设计的，两者不应同时使用。

### 集成选项

#### 选项 A: 完全替换
在 `SettingsView.vue` 中将 `ProvidersSettings` 路由替换为新的卡片视图组件。

#### 选项 B: 视图切换
在 `ProvidersSettings.vue` 中添加视图切换按钮，允许用户在"卡片视图"和"详细视图"之间切换。

```vue
<template>
  <div class="providers-settings">
    <div class="view-switcher">
      <button 
        :class="{ active: viewMode === 'card' }"
        @click="viewMode = 'card'"
      >
        卡片视图
      </button>
      <button 
        :class="{ active: viewMode === 'detail' }"
        @click="viewMode = 'detail'"
      >
        详细视图
      </button>
    </div>
    
    <ProvidersCardView 
      v-if="viewMode === 'card'"
      ... 
    />
    <ProvidersSettings 
      v-else 
      ...
    />
  </div>
</template>
```

---

## 响应式布局

卡片网格使用 CSS Grid 自动适配窗口大小:

```css
.providers-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 1.5rem;
}
```

- **小窗口** (< 600px): 1 列
- **中窗口** (600-900px): 2 列
- **大窗口** (> 900px): 3 列或更多

---

## 主题适配

组件自动适配三种主题，通过 CSS 变量系统实现:

### Love 主题
- 粉色强调色 (`#ec4899`)
- 粉色背景渐变
- 樱花浮动效果

### Dark 主题
- 蓝色强调色 (`#60a5fa`)
- 深色背景
- 低饱和度配色

### Default 主题
- 粉色强调色
- 白色/浅粉背景
- 简约配色

---

## 下一步工作

### 需要后端支持的功能

以下功能需要后端 Rust 代码支持:

1. **加载供应商列表**: `invoke('get_providers')`
2. **测试连接**: `invoke('test_provider_connection', {...})`
3. **保存配置**: `invoke('save_provider_config', {...})`
4. **删除供应商**: `invoke('delete_custom_provider', {...})`
5. **导入配置**: 需要定义新的 command

### 可选增强功能

1. **QR 码显示**: 如果供应商支持 QR 码配置，可添加 QR 码显示 Modal
2. **批量操作**: 添加批量导入/导出功能
3. **配置历史**: 记录配置变更历史
4. **智能推荐**: 根据用户位置推荐最优 API Base

---

## 故障排除

### 卡片不显示

检查:
1. `providers` 数组是否为空
2. CSS 是否正确加载 (检查浏览器 DevTools)
3. 主题变量是否正确定义

### 向导模态框不弹出

检查:
1. `v-model:open` 是否正确绑定
2. Teleport 是否工作正常 (Vue 3 特性)
3. z-index 是否被其他元素覆盖

### 样式异常

检查:
1. `styles.css` 是否已更新
2. CSS 变量是否在主题中定义
3. 是否有其他 CSS 规则覆盖

---

## 技术栈

- **Vue 3**: Composition API + `<script setup>`
- **TypeScript**: 完整类型定义
- **Tailwind CSS**: 工具类 + 自定义 CSS
- **lucide-vue-next**: 图标库
- **vue-i18n**: 国际化

---

## 贡献指南

如需修改或增强组件，请遵循:

1. 保持 Vue 3 Composition API 风格
2. 使用 TypeScript 类型定义
3. 遵循现有 CSS 变量系统
4. 添加中英文翻译
5. 更新本文档

---

## 相关文件

- [设计计划文档](./design-plan.md)
- [ProvidersSettings.vue](../../../src/components/settings/ProvidersSettings.vue)
- [styles.css](../../../src/styles.css)
- [zh.ts](../../../src/locales/zh.ts)
- [en.ts](../../../src/locales/en.ts)
