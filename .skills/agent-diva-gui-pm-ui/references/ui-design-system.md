# Agent Diva GUI 设计系统

## 技术栈

- **框架**：Vue 3 (Composition API) + TypeScript
- **样式**：Tailwind CSS + `src/styles.css` 自定义层
- **图标**：lucide-vue-next
- **国际化**：vue-i18n（en, zh）

## 主题

三种主题：`default`（浅色）、`dark`（深色）、`love`（粉色系）。

主题通过 `theme-*` 类作用于根容器，影响 `.app-shell`、`.chat-shell`、`.chat-bubble-*` 等。

### 色彩 token（love 主题示例）

| 用途 | 值 |
|------|-----|
| 主背景 | `linear-gradient(160deg, #fff5f7 0%, #ffe4ef 60%, #ffd6e8 100%)` |
| 标题栏 | `rgba(255, 240, 246, 0.85)` |
| 气泡用户 | `linear-gradient(135deg, #ffd3e1, #ffb4cc)` |
| 气泡助手 | `rgba(255, 255, 255, 0.95)` |
| 强调色 | `#ec4899`（pink-500） |

### 组件类

- `.app-shell` / `.app-titlebar` / `.app-emotion` / `.app-badge`
- `.chat-shell` / `.chat-input-bar` / `.chat-input` / `.chat-avatar` / `.chat-bubble-user` / `.chat-bubble-assistant`
- `.theme-dark.*` / `.theme-love.*` 覆盖上述默认色

## 布局

- 主窗口：800×600（可调），无边框或自定义标题栏
- 启动屏：400×300，居中，无装饰
- 拖拽区：`.drag-region` / `.no-drag`（Tauri 窗口）

## 组件层级

```
App.vue
└── NormalMode.vue
    ├── ChatView（对话列表 + 输入栏）
    ├── SettingsView（设置面板）
    ├── ConsoleView（控制台）
    └── CronTaskManagementView（Cron 管理）
```

## 消息类型

- `user`：用户消息
- `agent`：助手回复（含 reasoning、streaming、emotion）
- `system`：系统提示
- `tool`：工具调用（running/success/error）

## 国际化 key 结构

- `app.*`：应用级（welcome、errorPrefix、configUpdated 等）
- `chat.*`：对话（placeholder、clearChat、historySessions 等）
- `settings.*`：设置（title、general、providers 等）
- `emotion.*`：情绪状态

## 新增 UI 时注意

1. 使用现有 `theme-*` 与 `.chat-*` / `.app-*` 类保持一致性
2. 新文案加入 `locales/en.ts` 与 `locales/zh.ts`
3. 图标优先用 lucide-vue-next
4. 响应式：考虑 800px 宽度下的布局
