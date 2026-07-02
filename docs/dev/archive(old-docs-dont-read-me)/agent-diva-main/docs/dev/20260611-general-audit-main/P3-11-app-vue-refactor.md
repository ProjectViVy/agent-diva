# P3-11: App.vue 过重

## 问题描述

`agent-diva-gui/src/App.vue` 当前约 1384 行，`<script setup>` 从第 1 行持续到模板前，`<template>` 直到约第 1352 行才出现。该文件同时承担了类型定义、状态管理、会话缓存、配置读写、消息发送、流式事件监听、健康检查、欢迎向导协调、provider 配置解析和 UI 组合。

典型混合职责包括：

- 第 20 行起定义 `Message`、stream payload、provider config、session cache 等多组类型。
- 第 143 行起维护 `messages`、`config`、`toolsConfig`、`savedModels`、`sessions` 等全局状态。
- 第 253 行起解析 provider 原始配置并 patch 配置 JSON。
- 第 423 行起将后端 session message 映射为 UI message。
- 第 578 行起实现 `sendMessage`，直接调用 Tauri `invoke("send_message")`。
- 第 723 行起实现 session 刷新、恢复、加载、删除。
- 第 895 行起实现 `saveConfig`、`saveToolsConfig`、`saveChannelConfig`。
- 第 1031 行起在一个大型 `onMounted` 中完成启动加载、健康检查、session 恢复和多个事件监听。
- 模板层实际只组合 `WelcomeWizard` 与 `NormalMode`，但上方脚本承载了大量业务逻辑。

## 影响评估

- 可维护性影响：任何 chat、session、provider、tools、channel 或启动流程改动都需要进入同一个文件，冲突概率高。
- 测试影响：组合式逻辑没有抽成 composable，难以对 session 映射、缓存 TTL、provider 配置 patch、stream event reducer 做单元测试。
- 认知负担：模板很薄，但脚本过重，新开发者难以快速判断状态归属和事件边界。
- 回归风险：生命周期监听、localStorage watch、Tauri invoke 和消息状态更新交织，容易在修复一个流程时破坏另一个流程。

## 解决方案

按职责拆分为 composables、API adapter 和纯工具函数，保留 `App.vue` 只做顶层编排。

建议拆分：

```text
agent-diva-gui/src/
  composables/
    useChatRuntime.ts
    useSessionHistory.ts
    useProviderConfig.ts
    useToolConfig.ts
    useBackgroundEvents.ts
    useWelcomeFlow.ts
  utils/
    sessionMapping.ts
    providerConfigPatch.ts
    localSessionCache.ts
  api/
    desktop.ts
    chat.ts
```

拆分原则：

- 纯函数先移出：`extractChatId`、`mapBackendMessageToUi`、`extractProviderConfigsFromRaw`、`patchProviderConfigInRaw`。
- Tauri invoke 封装在 API 层：`sendMessage`、`stop_generation`、`get_sessions`、`get_session_history` 不直接散落在 `App.vue`。
- streaming event 处理抽成 reducer：输入 event payload 和当前 message state，输出下一状态。
- `App.vue` 只保留组件组合、composable 调用和事件转发。

示例：

```ts
export function useSessionHistory() {
  const sessions = ref<SessionInfo[]>([]);

  async function refreshSessions() {
    const fetched = await getSessions();
    sessions.value = mapSessions(fetched);
  }

  async function loadSession(sessionKey: string) {
    const history = await getSessionHistory(sessionKey);
    return history?.messages.map(mapBackendMessageToUi).filter(Boolean) ?? [];
  }

  return { sessions, refreshSessions, loadSession };
}
```

拆分过程中不建议同时大改 UI。先保持行为等价，再补测试。

## 验证方法

执行：

```powershell
cd agent-diva-gui
npm run typecheck
npm run build
cd ..
just fmt-check
just check
```

建议新增前端测试：

- `sessionMapping.ts`：角色映射、tool call 映射、时间戳解析。
- `providerConfigPatch.ts`：builtin provider 与 custom provider patch 逻辑。
- `localSessionCache.ts`：TTL 命中、过期、兼容 key。
- `useBackgroundEvents`：stream delta、tool start/end、error 去重、stop suppression。

验收标准：

- `App.vue` 显著缩小，只负责顶层组合。
- 原有 GUI 启动、发送、停止、加载历史、删除 session、保存配置流程保持行为一致。
- GUI smoke：启动应用，发送一条消息，停止一次生成，切换/加载历史会话。

## 优先级

P3
