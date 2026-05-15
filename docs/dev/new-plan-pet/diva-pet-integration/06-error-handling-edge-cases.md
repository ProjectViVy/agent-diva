# 06 - 错误处理与边界条件

> Diva Pet 模块的异常场景与容错策略

---

## 1. 错误处理总览

```
                        ┌──────────────┐
                        │  错误发生    │
                        └──────┬───────┘
                               │
                    ┌──────────▼──────────┐
                    │  是否为关键路径？    │
                    └──────┬─────┬────────┘
                           │     │
                    关键   │     │  非关键
                           │     │
              ┌────────────▼─┐ ┌─▼────────────┐
              │ 降级 + 通知  │ │ 静默降级/重试 │
              │ (toast/dialog)│ │ (console.warn)│
              └──────────────┘ └──────────────┘
```

---

## 2. Live2D 渲染错误处理

### 2.1 错误分类

| 错误类型 | 严重程度 | 用户可见 | 处理策略 |
|----------|----------|----------|----------|
| Cubism 5 Wasm 加载超时 | 🔴 Critical | ✅ | 显示错误页面 + 重试按钮 |
| WebGL context 不可用 | 🔴 Critical | ✅ | 降级为静态角色图 |
| .model3.json 解析失败 | 🟡 Warning | ✅ | Toast 错误 + 回退到默认模型 |
| .moc3 数据损坏 | 🟡 Warning | ✅ | Toast 错误 + 回退到默认模型 |
| 贴图加载失败 | 🟡 Warning | ✅ | 模型显示但无贴图（白色） |
| 着色器编译失败 | 🔴 Critical | ✅ | 降级到静态图 |
| WebGL context lost | 🟡 Warning | ❌ | 自动恢复（监听 contextrestored） |
| 帧渲染异常 | 🟢 Info | ❌ | 跳过该帧，继续循环 |

### 2.2 具体处理代码

```typescript
// features/diva-pet/components/DivaPetAvatar.vue

const errorState = ref<'none' | 'loading' | 'webgl-unavailable' | 'load-failed'>('none')
const errorMessage = ref('')

async function initLive2d() {
  errorState.value = 'loading'

  try {
    // Step 1: 检查 WebGL 支持
    const canvas = canvasRef.value!
    const gl = canvas.getContext('webgl', { alpha: true, premultipliedAlpha: true })
    if (!gl) {
      errorState.value = 'webgl-unavailable'
      errorMessage.value = 'WebGL 不可用，无法渲染 Live2D 角色'
      return
    }

    // Step 2: 加载 Cubism 5 Core
    await ensureCubism5CoreReady({ forceFrameworkReset: true })

    // Step 3: 加载模型文件
    const bundle = await loadLive2dModelBundle(props.model.relativeModelPath)

    // Step 4: 创建模型实例
    model.value = await createCubism5Model({
      bundle, gl,
      modelLabel: props.model.label,
      viewportWidth: canvas.width,
      viewportHeight: canvas.height,
    })

    errorState.value = 'none'
    emit('load-success')
    startRenderLoop()

  } catch (err) {
    console.error('[DivaPet] Live2D initialization failed:', err)
    errorState.value = 'load-failed'
    errorMessage.value = resolveLive2dErrorMessage(err)
    emit('load-error', new Error(errorMessage.value))

    // 尝试降级：使用默认模型重试一次
    tryFallbackModel()
  }
}

async function tryFallbackModel() {
  try {
    const defaultBundle = await loadLive2dModelBundle(DEFAULT_MODEL_PATH)
    model.value = await createCubism5Model({ /* ... */ })
    errorState.value = 'none'
    startRenderLoop()
  } catch {
    // 连默认模型也加载失败 → 保持 error 状态
  }
}
```

### 2.3 WebGL Context Lost 处理

```typescript
canvas.addEventListener('webglcontextlost', (event) => {
  event.preventDefault()  // 允许上下文恢复
  stopRenderLoop()
  console.warn('[DivaPet] WebGL context lost')
})

canvas.addEventListener('webglcontextrestored', () => {
  console.log('[DivaPet] WebGL context restored, reinitializing...')
  initLive2d()  // 重新初始化
})
```

---

## 3. ASR（语音识别）错误处理

### 3.1 错误分类

| SpeechRecognition 错误 | 含义 | 处理 |
|------------------------|------|------|
| `no-speech` | 未检测到语音 | 静默忽略，自动重启监听 |
| `aborted` | 用户主动停止 | 正常行为，不处理 |
| `audio-capture` | 无麦克风设备 | 禁用 ASR，显示提示 |
| `not-allowed` | 麦克风权限被拒 | 禁用 ASR，引导用户设置 |
| `service-not-allowed` | 系统禁止语音服务 | 禁用 ASR |
| `network` | 网络不可用 | 重试 3 次后禁用 |
| `bad-grammar` | 语法错误 | 重试 |
| `language-not-supported` | 语言不支持 | 切换为英文重试 |

### 3.2 实现

```typescript
// features/diva-pet/composables/useVoiceInput.ts

function handleRecognitionError(event: SpeechRecognitionErrorEventLike) {
  switch (event.error) {
    case 'no-speech':
      // 静默忽略，已在 onend 中自动重启
      return

    case 'aborted':
      return

    case 'audio-capture':
      error.value = '未检测到可用麦克风。请检查设备连接。'
      isEnabled.value = false
      break

    case 'not-allowed':
    case 'service-not-allowed':
      error.value = '麦克风权限未授予。请在系统设置中允许 Agent Diva 使用麦克风。'
      isEnabled.value = false
      break

    case 'network':
      retryCount++
      if (retryCount >= 3) {
        error.value = '语音识别服务暂时不可用，请检查网络连接。'
        isEnabled.value = false
      }
      break

    default:
      error.value = `语音识别出错：${event.error}`
  }
}
```

### 3.3 TTS 播放时自动暂停 ASR

```typescript
// 避免 TTS 播报被麦克风录入
watch(isSpeaking, (speaking) => {
  if (speaking) {
    pauseFor(TTS_ESTIMATED_DURATION_MS)
  }
})
```

---

## 4. TTS（语音合成）错误处理

### 4.1 降级链

```
1️⃣ CosyVoice2 声音克隆
   ├── 成功 → 播放
   ├── 403 (API Key 无效) → 跳到步骤 2
   ├── 404 (克隆声音过期) → 重建克隆 → 重试
   ├── 408/429/5xx → 重试 1 次 → 失败则跳到步骤 2
   └── 超时 (30s) → 跳到步骤 2

2️⃣ 标准 OpenAI/TTS API
   ├── 成功 → 播放
   └── 失败 → 跳到步骤 3

3️⃣ 浏览器 SpeechSynthesis
   ├── 成功 → 播放
   └── 失败 → 静默（无语音输出）
```

### 4.2 错误类型与处理

| 错误 | 处理 | 用户感知 |
|------|------|----------|
| API Key 无效 | 跳转到 browser TTS | Toast 提示配置检查 |
| 网络超时 | 重试 1 次 → 降级 | 无感知（降级透明） |
| 音频解码失败 | 跳过该条 | 继续下一条 |
| 播放被中断（新消息） | 停止当前，播放新 | 正常行为 |
| SpeechSynthesis 不可用 | 静默 | 无感知 |

### 4.3 TTSService 错误处理增强

```typescript
// tts-service.ts 中已有的错误处理（可直接复用）

// 1. 请求级重试
const VOICE_SYNTHESIS_REQUEST_POLICY: TTSRequestPolicy = {
  maxRetries: 1,       // 最多重试 1 次
  purpose: "voice synthesis request",
  timeoutMs: 30000      // 30 秒超时
}

// 2. 可重试状态码
const DEFAULT_RETRYABLE_STATUSES = new Set([408, 425, 429, 500, 502, 503, 504])

// 3. 声音克隆 URI 失效 → 自动重建
if (status === 404 || status === 410) {
  // 标记为 clone_voice_invalid → 触发 rebuildCloneVoiceUri
}

// 4. 浏览器 TTS 兜底
private async synthesizeWithBrowser(request: TTSRequest): Promise<TTSResponse | null> {
  if (!window.speechSynthesis) return null
  // ... SpeechSynthesisUtterance
}
```

---

## 5. 边界条件

### 5.1 首次使用

| 场景 | 预期行为 |
|------|----------|
| 无 config.json | `pet.enabled` 默认 `false`，Diva Pet 侧边栏不显示 |
| 无 API Key | TTS 使用浏览器 SpeechSynthesis（免费） |
| 无 Live2D 模型 | 显示静态默认角色图 |
| 无麦克风 | ASR 按钮显示为禁用状态 |

### 5.2 资源限制

| 场景 | 预期行为 |
|------|----------|
| GPU 内存不足 | WebGL 纹理创建失败 → 降级到静态图 |
| 模型文件过大 (>100MB) | 显示加载进度条 |
| 多个 WebGL context | 仅创建 1 个 context，多个模型共享 |
| 窗口最小化 | 暂停 Live2D 渲染循环 |

### 5.3 并发场景

| 场景 | 预期行为 |
|------|----------|
| 快速连续发送消息 | TTS 仅播放最后一条（前一条被 cancel） |
| 同时 ASR + TTS | ASR 自动暂停直到 TTS 完成 |
| 模型切换 + 渲染中 | 取消当前渲染，加载新模型 |
| 会话切换 + TTS 播放中 | 停止播放 |

### 5.4 窗口管理

| 场景 | 预期行为 |
|------|----------|
| Tauri 多窗口（pet 独立窗口） | 可选，仅主窗口渲染 Live2D |
| 窗口隐藏到托盘 | 暂停渲染循环 |
| 窗口还原 | 恢复渲染循环 |
| DPI 变化 | 重新计算 Canvas 尺寸和视口 |

### 5.5 极端输入

| 场景 | 处理 |
|------|------|
| TTS 文本为空 | 跳过，不调用 API |
| TTS 文本超长 (>5000 字) | 分段合成（最大 1000 字/段） |
| 模型路径包含特殊字符 | URL 编码处理 |
| API 返回非 MP3 格式 | 检查 Content-Type，仅处理 audio/* |
| 语音识别返回空字符串 | 忽略，不发送消息 |

---

## 6. 日志策略

```typescript
// 层级化的日志
const LOG_PREFIX = '[DivaPet]'

// 错误（用户可见或有损体验）
console.error(`${LOG_PREFIX} Live2D initialization failed:`, error)

// 警告（自动降级，用户可能无感知）
console.warn(`${LOG_PREFIX} CosyVoice2 unavailable, falling back to browser TTS`)

// 信息（正常流程）
console.log(`${LOG_PREFIX} Model loaded: ${model.label}`)

// 调试（仅开发模式）
if (import.meta.env.DEV) {
  console.debug(`${LOG_PREFIX} Frame ${frameCount}: ${fps} fps`)
}
```

---

## 7. 用户提示文案

```typescript
// locales/zh.ts 新增

export default {
  pet: {
    error: {
      webglUnavailable: '您的设备不支持 WebGL，无法显示 Live2D 角色。',
      modelLoadFailed: '角色模型加载失败，已切换为默认角色。',
      modelNotFound: '未找到 Live2D 模型文件。请先导入模型。',
      micPermissionDenied: '麦克风权限未授予。请在系统设置中允许使用麦克风。',
      micNotFound: '未检测到可用麦克风。请检查设备连接。',
      asrNetworkError: '语音识别服务暂时不可用，请检查网络后再试。',
      ttsApiKeyInvalid: 'TTS API Key 无效，已切换为系统语音。请在设置中重新配置。',
      ttsNetworkError: '语音合成失败，已切换为本地语音。',
    },
    hint: {
      howToImportModel: '将 Live2D 模型文件夹拖拽到此处，或点击导入。',
      supportedFormats: '支持的格式：.model3.json（Cubism 5）',
      asrGuide: '点击麦克风按钮开始语音输入',
    }
  }
}
```
