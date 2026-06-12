# Task: CC-P3 - Create CompactionSettings GUI component

Working directory: C:/Users/Administrator/Desktop/morediva/agent-diva-pro
Branch: feature/context-compaction

## Goal
Add a Context Compaction settings panel to the GUI Settings view.

## Files to create
1. `agent-diva-gui/src/components/settings/CompactionSettings.vue`

## Files to modify
2. `agent-diva-gui/src/components/SettingsView.vue` - register new settings view
3. `agent-diva-gui/src/components/settings/SettingsDashboard.vue` - add dashboard card
4. `agent-diva-gui/src/locales/en.ts` - add i18n keys
5. `agent-diva-gui/src/locales/zh.ts` - add i18n keys

## Component Requirements (CompactionSettings.vue)

### Script setup pattern
Follow the same pattern as `SandboxSettingsSection.vue` and `SelfEvolutionSettings.vue`:
- `<script setup lang="ts">` with Vue 3 Composition API
- Use `ref`, `computed`, `onMounted`, `watch` from Vue
- Use `useI18n` for translations
- Use `invoke` from `@tauri-apps/api/core` for backend calls
- Use `showAppToast` from `../../utils/appToast` for notifications

### Config interface
```typescript
interface CompactionConfig {
  max_tokens: number;              // Default: 180000
  compact_threshold_ratio: number; // Default: 0.80 (range 0.1-1.0)
  keep_recent_count: number;       // Default: 10 (range 1-50)
}
```

### UI sections

**Section 1: Budget Status Display**
- Show a progress bar with current context pressure (color: green < 60%, yellow 60-80%, red > 80%)
- Display: history_estimated tokens / history_budget tokens
- Display: pressure_ratio as percentage
- Display: should_compact status badge (green "OK" or orange "Compact Recommended")
- Try to load budget status via `invoke('get_budget_status')` - wrap in try/catch, on error show "Budget status unavailable (backend not connected)" and disable the live display
- Add a refresh button to re-fetch budget status

**Section 2: Configuration**
- max_tokens: number input (range 10000-500000, step 10000), with label "Maximum Context Tokens"
- compact_threshold_ratio: range slider (0.1-1.0, step 0.05), showing percentage label, with label "Compaction Threshold"
- keep_recent_count: number input (range 1-50, step 1), with label "Keep Recent Messages"

**Section 3: Manual Compaction**
- A "Run Compaction" button that sends `/compact` via the chat API
- Use `invoke('send_message', { message: '/compact', channel: null, chatId: null, attachments: null, streamRequestId: crypto.randomUUID() })`
- Show loading spinner while running
- Show success/error toast after
- Disable button if budget status shows should_compact = false (with tooltip "No compaction needed")

### Settings persistence
- Save config to localStorage key `agent-diva-compaction-config`
- Load on mount, merge with defaults
- Save on change (debounced 500ms via watch)
- Show a "Reset to Defaults" button

### Styling
- Use Tailwind CSS classes (same as other settings components)
- Wrap in `<div class="space-y-6 p-6">` 
- Use card-style sections with `<div class="bg-white rounded-xl border border-gray-200 p-6 shadow-sm">`
- Use the lucide `Minimize2` icon for the component import (or `Archive` / `Shrink` if not available)

## SettingsView.vue changes

1. Add `'compaction'` to the `SettingsSubview` type union
2. Import: `import CompactionSettings from './settings/CompactionSettings.vue';`
3. Add to `pageTitle` computed: `compaction: t('dashboard.compaction')`
4. Add to template after the `sandbox` section:
```vue
<div v-else-if="currentView === 'compaction'">
  <CompactionSettings />
</div>
```

## SettingsDashboard.vue changes

1. Add `'compaction'` to the emit type
2. Import `Minimize2` (or chosen icon) from lucide-vue-next
3. Add card to the `cards` array (after sandbox, before pet):
```typescript
{ id: 'compaction', icon: Minimize2, title: t('dashboard.compaction'), desc: t('dashboard.compactionDesc') },
```

## i18n keys to add

### en.ts - add to `dashboard` section:
```
compaction: 'Compaction',
compactionDesc: 'Configure context window compaction and budget management',
```

### en.ts - add new top-level section `compaction`:
```
compaction: {
  title: 'Context Compaction',
  budgetStatus: 'Budget Status',
  historyTokens: 'History Tokens',
  pressureRatio: 'Pressure Ratio',
  status: 'Status',
  statusOk: 'OK',
  statusCompact: 'Compact Recommended',
  unavailable: 'Budget status unavailable (backend not connected)',
  refresh: 'Refresh',
  config: 'Configuration',
  maxTokens: 'Maximum Context Tokens',
  maxTokensDesc: 'Total token budget for the context window',
  thresholdRatio: 'Compaction Threshold',
  thresholdRatioDesc: 'Trigger compaction when history reaches this percentage of budget',
  keepRecent: 'Keep Recent Messages',
  keepRecentDesc: 'Number of recent messages to always preserve during compaction',
  manualCompact: 'Manual Compaction',
  runCompact: 'Run Compaction',
  compactRunning: 'Running...',
  compactSuccess: 'Compaction completed successfully',
  compactError: 'Compaction failed',
  noCompactNeeded: 'No compaction needed',
  resetDefaults: 'Reset to Defaults',
  percent: '%',
},
```

### zh.ts - add to `dashboard` section:
```
compaction: '上下文压缩',
compactionDesc: '配置上下文窗口压缩与预算管理',
```

### zh.ts - add new top-level section `compaction`:
```
compaction: {
  title: '上下文压缩',
  budgetStatus: '预算状态',
  historyTokens: '历史 Token 数',
  pressureRatio: '压力比',
  status: '状态',
  statusOk: '正常',
  statusCompact: '建议压缩',
  unavailable: '预算状态不可用（后端未连接）',
  refresh: '刷新',
  config: '配置',
  maxTokens: '最大上下文 Token 数',
  maxTokensDesc: '上下文窗口的总 Token 预算',
  thresholdRatio: '压缩阈值',
  thresholdRatioDesc: '当历史消息达到预算的此百分比时触发压缩',
  keepRecent: '保留最近消息数',
  keepRecentDesc: '压缩时始终保留的最近消息条数',
  manualCompact: '手动压缩',
  runCompact: '执行压缩',
  compactRunning: '执行中...',
  compactSuccess: '压缩完成',
  compactError: '压缩失败',
  noCompactNeeded: '无需压缩',
  resetDefaults: '恢复默认',
  percent: '%',
},
```

## PROJECT RULES (MUST FOLLOW)
- Vue 3 + `<script setup lang="ts">` + Composition API only (no Options API)
- Tailwind CSS for styling (no custom CSS unless absolutely necessary)
- lucide-vue-next for icons
- vue-i18n `useI18n()` for all user-visible strings
- Tauri `invoke` for backend calls, always wrapped in try/catch
- No external dependencies (no new npm packages)
- Follow existing file naming: PascalCase for components
- Keep component self-contained (no new store files)
- All i18n keys must be added to BOTH en.ts and zh.ts

## Verification
After making changes:
1. Check that the TypeScript compiles: `cd agent-diva-gui && npx vue-tsc --noEmit 2>&1 | head -20` (or just check for import errors)
2. Verify all files are syntactically correct
3. Run `git diff --stat` to show what changed

Final report must end with the line CC_P3_COMPACTION_SETTINGS_DONE
