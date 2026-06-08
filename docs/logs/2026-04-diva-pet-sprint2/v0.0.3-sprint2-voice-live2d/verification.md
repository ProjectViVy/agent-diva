# Sprint 2 — 验证

## 技术验证

### 代码架构验证

| 检查项 | 方法 | 结果 |
|--------|------|------|
| TTS 服务框架无关性 | 审查 imports — 无 React/Vue/Tauri 依赖 | ✅ |
| VoiceFileReader 注入模式 | 审查 `setVoiceFileReader()` + 两个调用点优雅降级 | ✅ |
| useVoiceInput React→Vue3 转换 | 审查：useState→ref, useEffect→watch, useCallback→plain fn | ✅ |
| useVoicePlayer 消息监听 | 审查：watch(messages, ..., {deep:false}) + lastMessageCount 增量 | ✅ |
| cubism5-core 路径适配 | 审查：`../../../vendor/cubism5-framework/`, `data-diva-pet-cubism5-core` | ✅ |
| cubism5-model AniPet 依赖移除 | 审查：无 customization-types 导入, LoadedLive2dModelBundle 内联定义 | ✅ |
| DivaPetAvatar 无 Tauri/React | 审查：无 Tauri API, 无 React hooks, Vue3 Composition API | ✅ |
| DivaPetView 渲染器切换 | 审查：v-if="renderer === 'vrm'" / v-else-if="renderer === 'live2d'" | ✅ |
| index.ts 导出完整性 | 审查：所有新增组件/composable/service/type 均已导出 | ✅ |

### Vendor 文件复制验证

| 检查项 | 方法 | 结果 |
|--------|------|------|
| cubism5-framework (47文件) | robocopy 日志 + Get-ChildItem 计数 | ✅ |
| shader 文件 (5.vert + 8.frag) | robocopy 日志 | ✅ |
| CubismCore (2文件) | copy 命令 + 文件存在性 | ✅ |
| src/vendor/ 结构完整 | robocopy 至 src/vendor/cubism5-framework/ | ✅ |

### 类型安全

| 检查项 | 结果 |
|--------|------|
| `@ts-nocheck` 仅用于 vendor/cubism5 文件 | ✅ (cubism5-core.ts, cubism5-model.ts) |
| 业务代码无 `as any` / `@ts-ignore` | ✅ |
| LSP 诊断 | ⚠️ typescript-language-server 未安装 |

## 已知限制

- **LSP 不可用**：Windows 环境未安装 `typescript-language-server`。建议通过 `vue-tsc --noEmit` 进行类型检查。
- **Live2D 模型加载路径**：DivaPetAvatar 的 `loadModel()` 方法已实现但尚未被上层调用，需 Tauri 命令 `pet_load_live2d_bundle` 支持。
- **VRM 口型同步**：useVrmMouthSync 已实现但未接入 Three.js 渲染循环，需在后续迭代中连线。

## 构建说明

由于 LSP 不可用，完整的类型检查需要在 GUI 目录下执行：

```bash
cd agent-diva-gui
pnpm install  # 如忘记执行
npx vue-tsc --noEmit
```

Vite 构建：

```bash
cd agent-diva-gui
npx vite build
```
