# 11 - 重要代码示例

> Diva Pet 模块关键代码片段的参考实现
>
> **参考源文件索引**:
> - Cubism5 核心: [`AniPet/apps/desktop/src/components/live2d-avatar/cubism5-core.ts`](../../../../../AniPet/apps/desktop/src/components/live2d-avatar/cubism5-core.ts):1-379
> - Cubism5 模型: [`AniPet/apps/desktop/src/components/live2d-avatar/cubism5-model.ts`](../../../../../AniPet/apps/desktop/src/components/live2d-avatar/cubism5-model.ts):1-1524
> - Live2D 渲染器: [`AniPet/apps/desktop/src/components/live2d-avatar/Live2DAvatarRenderer.tsx`](../../../../../AniPet/apps/desktop/src/components/live2d-avatar/Live2DAvatarRenderer.tsx):1-1373
> - TTS 服务: [`AniPet/apps/desktop/src/features/voice/tts-service.ts`](../../../../../AniPet/apps/desktop/src/features/voice/tts-service.ts):1-1048
> - 语音输入: [`AniPet/apps/desktop/src/features/voice/use-voice-input.ts`](../../../../../AniPet/apps/desktop/src/features/voice/use-voice-input.ts):1-402
> - 语音播放: [`AniPet/apps/desktop/src/features/voice/use-voice-player.ts`](../../../../../AniPet/apps/desktop/src/features/voice/use-voice-player.ts):1-204

---

## 1. Vue 3 组件：DivaPetAvatar.vue

```vue
<script setup lang="ts">
/**
 * DivaPetAvatar.vue — Live2D 角色渲染组件
 *
 * 基于 AniPet 的 Live2DAvatarRenderer.tsx 改写为 Vue 3 组件。
 * 负责 WebGL Canvas 的创建、Live2D 模型的加载与渲染循环。
 */
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'
import { ensureCubism5CoreReady, getRecentCubism5Logs } from '../live2d/cubism5-core'
import { createCubism5Model, type Cubism5ModelRuntime } from '../live2d/cubism5-model'
import { loadLive2dModelBundle, type LoadedLive2dModelBundle } from '../services/model-loader'
import type { Live2dModelOption } from '../types'

// ── Constants ──────────────────────────────────────────────
const CANVAS_WIDTH = 360
const CANVAS_HEIGHT = 460
const QUALITY_SCALE_FACTOR = 1.5
const FRAME_INTERVAL_ACTIVE_MS = 16.67
const FRAME_INTERVAL_IDLE_MS = 33.33
const FRAME_INTERVAL_TOLERANCE = 0.85

// ── Props & Emits ──────────────────────────────────────────
interface Props {
  model: Live2dModelOption
  scale?: number
  offsetX?: number
  offsetY?: number
  emotion?: 'happy' | 'neutral' | 'shy'
  live2dExpression?: string | null
  live2dMotionGroup?: string | null
}

const props = withDefaults(defineProps<Props>(), {
  scale: 0.72,
  offsetX: 0,
  offsetY: 0,
  emotion: 'neutral',
})

const emit = defineEmits<{
  (e: 'load-start'): void
  (e: 'load-success'): void
  (e: 'load-error', error: Error): void
  (e: 'head-activate'): void
  (e: 'body-activate'): void
  (e: 'drag-start'): void
  (e: 'drag-end'): void
}>()

// ── State ──────────────────────────────────────────────────
const canvasRef = ref<HTMLCanvasElement | null>(null)
const loadState = ref<'idle' | 'loading' | 'ready' | 'error'>('idle')
const loadError = ref<string | null>(null)

let modelRuntime: Cubism5ModelRuntime | null = null
let glContext: WebGLRenderingContext | null = null
let rafId = 0
let lastFrameTime = 0
let forceHighFpsUntil = 0

// ── Model Loading ──────────────────────────────────────────
async function initializeModel() {
  const canvas = canvasRef.value
  if (!canvas) return

  loadState.value = 'loading'
  emit('load-start')

  try {
    // 1. Ensure Cubism 5 runtime
    await ensureCubism5CoreReady({ forceFrameworkReset: true })

    // 2. Set up WebGL
    const dpr = window.devicePixelRatio || 1
    const totalScale = dpr * QUALITY_SCALE_FACTOR
    canvas.width = Math.round(CANVAS_WIDTH * totalScale)
    canvas.height = Math.round(CANVAS_HEIGHT * totalScale)

    const gl = canvas.getContext('webgl', {
      alpha: true,
      antialias: true,
      premultipliedAlpha: true,
      preserveDrawingBuffer: true,
    })
    if (!gl) throw new Error('WebGL 不可用')
    glContext = gl

    // 3. Load model bundle (via Tauri IPC)
    const bundle = await loadLive2dModelBundle(props.model.relativeModelPath)

    // 4. Create Live2D model instance
    modelRuntime = await createCubism5Model({
      bundle,
      gl,
      modelLabel: props.model.label,
      viewportWidth: canvas.width,
      viewportHeight: canvas.height,
    })

    // 5. Apply viewport
    modelRuntime.setViewportOptions(props.scale, props.offsetX, props.offsetY)

    loadState.value = 'ready'
    emit('load-success')
    startRenderLoop()
  } catch (err) {
    console.error('[DivaPetAvatar] Init failed:', err)
    loadState.value = 'error'
    loadError.value = err instanceof Error ? err.message : String(err)
    emit('load-error', new Error(loadError.value))
  }
}

// ── Render Loop ────────────────────────────────────────────
function startRenderLoop() {
  lastFrameTime = performance.now()
  rafId = requestAnimationFrame(renderFrame)
}

function renderFrame(now: number) {
  if (!modelRuntime) return

  // Adaptive frame rate
  const elapsed = now - lastFrameTime
  const isActive =
    modelRuntime.isAnimating() || now < forceHighFpsUntil
  const target = isActive ? FRAME_INTERVAL_ACTIVE_MS : FRAME_INTERVAL_IDLE_MS

  if (elapsed < target * FRAME_INTERVAL_TOLERANCE) {
    rafId = requestAnimationFrame(renderFrame)
    return
  }

  const delta = lastFrameTime ? Math.min((now - lastFrameTime) / 1000, 0.05) : 1 / 60
  lastFrameTime = now

  try {
    modelRuntime.render(delta)
  } catch (err) {
    console.error('[DivaPetAvatar] Render error:', err)
  }

  rafId = requestAnimationFrame(renderFrame)
}

function stopRenderLoop() {
  if (rafId) {
    cancelAnimationFrame(rafId)
    rafId = 0
  }
}

// ── Expression & Motion Update ──────────────────────────────
watch(() => props.live2dExpression, (expr) => {
  if (!modelRuntime) return
  if (expr) {
    forceHighFpsUntil = performance.now() + 2000
    modelRuntime.setExpression(expr)
  } else {
    modelRuntime.clearExpression()
  }
})

watch(() => props.live2dMotionGroup, (group) => {
  if (!modelRuntime) return
  if (group) {
    forceHighFpsUntil = performance.now() + 2000
    modelRuntime.setDesiredMotionGroup(group)
  }
})

watch(() => props.scale, (s) => {
  modelRuntime?.setViewportOptions(s, props.offsetX, props.offsetY)
})

// ── Lifecycle ──────────────────────────────────────────────
onMounted(() => {
  initializeModel()

  // Pause when tab hidden
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) stopRenderLoop()
    else startRenderLoop()
  })
})

onUnmounted(() => {
  stopRenderLoop()
  modelRuntime?.dispose()
  modelRuntime = null
  glContext?.getExtension('WEBGL_lose_context')?.loseContext()
  glContext = null
})

// ── Visibility helpers ─────────────────────────────────────
const isReady = computed(() => loadState.value === 'ready')
const isError = computed(() => loadState.value === 'error')
</script>

<template>
  <div class="diva-pet-avatar">
    <!-- Loading State -->
    <div v-if="loadState === 'loading'" class="diva-pet-avatar__loading">
      <div class="animate-spin w-8 h-8 border-2 border-violet-500 border-t-transparent rounded-full" />
      <span class="text-sm text-gray-400 mt-2">角色加载中...</span>
    </div>

    <!-- Error State -->
    <div v-else-if="loadState === 'error'" class="diva-pet-avatar__error">
      <span class="text-red-400">{{ loadError }}</span>
      <button class="mt-2 text-violet-400 underline" @click="initializeModel">重试</button>
    </div>

    <!-- Canvas (hidden while loading/error) -->
    <canvas
      ref="canvasRef"
      class="diva-pet-avatar__canvas"
      :class="{ 'opacity-0': !isReady }"
      :style="{ width: CANVAS_WIDTH + 'px', height: CANVAS_HEIGHT + 'px' }"
    />
  </div>
</template>

<style scoped>
.diva-pet-avatar {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}

.diva-pet-avatar__canvas {
  transition: opacity 0.3s ease;
}

.diva-pet-avatar__loading,
.diva-pet-avatar__error {
  position: absolute;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}
</style>
```

---

## 2. Vue Composable：useVoiceInput.ts

```typescript
/**
 * useVoiceInput — 语音输入 Composable
 *
 * 将 AniPet 的 React Hook useVoiceInput 改写为 Vue 3 Composable。
 * 封装浏览器 Web Speech API，提供响应式的语音监听状态。
 */
import { ref, computed, onUnmounted } from 'vue'

interface UseVoiceInputOptions {
  language?: string
  onRecognizedText: (text: string) => void
  onPreviewText?: (text: string) => void
  isSuspended?: () => boolean
}

interface UseVoiceInputReturn {
  isListening: ReturnType<typeof ref<boolean>>
  isSupported: ReturnType<typeof computed<boolean>>
  isProcessing: ReturnType<typeof ref<boolean>>
  error: ReturnType<typeof ref<string | null>>
  start: () => void
  stop: () => void
  toggle: () => void
}

// ── Helpers ──────────────────────────────────────────────
function getSpeechRecognitionConstructor() {
  return (window as any).SpeechRecognition
    ?? (window as any).webkitSpeechRecognition
    ?? null
}

function normalizeTranscript(text: string): string {
  return text.replace(/\s+/g, ' ').trim()
}

// ── Composable ───────────────────────────────────────────
export function useVoiceInput(options: UseVoiceInputOptions): UseVoiceInputReturn {
  const SpeechRecognition = getSpeechRecognitionConstructor()

  const isListening = ref(false)
  const isProcessing = ref(false)
  const error = ref<string | null>(null)
  const isSupported = computed(() => SpeechRecognition !== null)

  let recognition: InstanceType<typeof SpeechRecognition> | null = null
  let restartTimer: number | null = null

  function createRecognition() {
    if (!SpeechRecognition) return null

    const rec = new SpeechRecognition()
    rec.continuous = false
    rec.interimResults = true
    rec.lang = options.language ?? 'zh-CN'
    rec.maxAlternatives = 1

    rec.onstart = () => {
      isListening.value = true
      error.value = null
    }

    rec.onresult = (event: SpeechRecognitionEvent) => {
      if (isProcessing.value) return

      for (let i = event.resultIndex; i < event.results.length; i++) {
        const result = event.results[i]
        if (result.isFinal) {
          const text = normalizeTranscript(result[0].transcript)
          if (text) {
            isProcessing.value = true
            options.onRecognizedText(text)
          }
        }
      }
    }

    rec.onerror = (event: SpeechRecognitionErrorEvent) => {
      if (event.error === 'no-speech' || event.error === 'aborted') return
      if (event.error === 'not-allowed') {
        error.value = '麦克风权限未授予'
        stop()
        return
      }
      error.value = `语音识别错误: ${event.error}`
    }

    rec.onend = () => {
      isListening.value = false
      isProcessing.value = false

      // Auto-restart
      if (!options.isSuspended?.()) {
        restartTimer = window.setTimeout(() => {
          if (!options.isSuspended?.() && !isListening.value) {
            start()
          }
        }, 420)
      }
    }

    return rec
  }

  function start() {
    if (!SpeechRecognition) {
      error.value = '当前环境不支持语音输入'
      return
    }

    recognition = createRecognition()
    try {
      recognition?.start()
    } catch (err) {
      console.warn('[VoiceInput] Start failed:', err)
    }
  }

  function stop() {
    if (restartTimer) {
      clearTimeout(restartTimer)
      restartTimer = null
    }
    recognition?.stop()
    recognition = null
    isListening.value = false
    isProcessing.value = false
  }

  function toggle() {
    if (isListening.value) stop()
    else start()
  }

  onUnmounted(() => {
    stop()
  })

  return {
    isListening,
    isSupported,
    isProcessing,
    error,
    start,
    stop,
    toggle,
  }
}
```

---

## 3. Tauri Rust Command: 加载 Live2D 模型

```rust
// src-tauri/src/commands.rs 新增

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Live2dModelFile {
    pub relative_path: String,
    pub base64_data: String,
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Live2dModelBundle {
    pub model_relative_path: String,
    pub files: Vec<Live2dModelFile>,
}

/// 递归读取目录中所有文件，编码为 base64
fn collect_directory_files(
    dir: &PathBuf,
    base_dir: &PathBuf
) -> Result<Vec<Live2dModelFile>, String> {
    let mut files = Vec::new();

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            let relative = path.strip_prefix(base_dir)
                .map_err(|e| format!("路径解析失败: {}", e))?;

            let bytes = fs::read(path)
                .map_err(|e| format!("读取文件失败 {}: {}", path.display(), e))?;

            let content_type = match path.extension().and_then(|e| e.to_str()) {
                Some("json") => "application/json",
                Some("png")  => "image/png",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("moc3") => "application/octet-stream",
                _ => "application/octet-stream",
            };

            files.push(Live2dModelFile {
                relative_path: relative.to_string_lossy().replace('\\', "/"),
                base64_data: BASE64.encode(&bytes),
                content_type: content_type.to_string(),
            });
        }
    }

    Ok(files)
}

#[tauri::command]
async fn pet_load_live2d_model(
    model_path: String,
    app_handle: tauri::AppHandle,
) -> Result<Live2dModelBundle, String> {
    // 解析模型目录路径
    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| format!("无法获取资源目录: {}", e))?;

    let full_path = resource_dir.join(&model_path);
    let model_dir = full_path.parent()
        .ok_or("无效的模型路径：无法获取父目录")?;

    if !model_dir.exists() {
        return Err(format!("模型目录不存在: {}", model_dir.display()));
    }

    let files = collect_directory_files(&model_dir.to_path_buf(), &resource_dir)?;

    Ok(Live2dModelBundle {
        model_relative_path: model_path,
        files,
    })
}

#[tauri::command]
async fn pet_list_live2d_models(
    app_handle: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| format!("无法获取资源目录: {}", e))?;

    let live2d_dir = resource_dir.join("live2d_resource");
    if !live2d_dir.exists() {
        return Ok(Vec::new());
    }

    let mut models = Vec::new();
    for entry in walkdir::WalkDir::new(&live2d_dir)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_name().to_string_lossy().ends_with(".model3.json") {
            let relative = entry.path()
                .strip_prefix(&resource_dir)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            models.push(relative);
        }
    }

    Ok(models)
}

// 在 main.rs 或 lib.rs 中注册:
// .invoke_handler(tauri::generate_handler![
//     ...existing_commands,
//     pet_load_live2d_model,
//     pet_list_live2d_models,
// ])
```

---

## 4. 集成点：Agent 回复流 → TTS

```typescript
// features/diva-pet/composables/useVoicePlayer.ts

import { ref, onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { ttsService, type TTSVoiceConfig } from '../services/tts-service'

export function useVoicePlayer(voiceConfig: () => TTSVoiceConfig) {
  const isPlaying = ref(false)
  let unlistenReply: UnlistenFn | null = null

  onMounted(async () => {
    // 监听 Agent 回复完成事件
    unlistenReply = await listen<{ request_id: string; data: string }>(
      'agent-response-complete',
      (event) => {
        const text = event.payload.data
        if (text?.trim()) {
          speak(text.trim())
        }
      }
    )
  })

  async function speak(text: string) {
    const config = voiceConfig()
    if (!config.enabled) return

    isPlaying.value = true
    try {
      await ttsService.speakText(text, config)
    } catch (err) {
      console.warn('[VoicePlayer] TTS failed:', err)
    } finally {
      isPlaying.value = false
    }
  }

  function stop() {
    ttsService.stopPlayback()
    isPlaying.value = false
  }

  onUnmounted(() => {
    unlistenReply?.()
    stop()
  })

  return { isPlaying, speak, stop }
}
```

---

## 5. 配置加载：Pet Config

```typescript
// features/diva-pet/services/pet-config.ts

import { invoke } from '@tauri-apps/api/core'

export interface PetConfig {
  enabled: boolean
  live2d: {
    modelPath: string
    scale: number
    offsetX: number
    offsetY: number
    renderQuality: 'low' | 'medium' | 'high'
  }
  voice: {
    enabled: boolean
    tts: {
      provider: string
      apiKey: string
      baseUrl: string
      model: string
      referenceVoice: string
      speed: number
    }
  }
}

const DEFAULT_PET_CONFIG: PetConfig = {
  enabled: false,
  live2d: {
    modelPath: 'live2d_resource/default/mao_pro.model3.json',
    scale: 0.72,
    offsetX: 0,
    offsetY: 0,
    renderQuality: 'medium',
  },
  voice: {
    enabled: true,
    tts: {
      provider: 'browser',
      apiKey: '',
      baseUrl: 'https://api.siliconflow.cn/v1',
      model: 'FunAudioLLM/CosyVoice2-0.5B',
      referenceVoice: '',
      speed: 1.0,
    },
  },
}

let cachedConfig: PetConfig | null = null

export async function loadPetConfig(): Promise<PetConfig> {
  try {
    // 通过 Tauri IPC 读取 config.json 中的 pet section
    const raw = await invoke<string>('load_config')
    const parsed = JSON.parse(raw)
    const petSection = parsed?.pet

    if (petSection) {
      cachedConfig = { ...DEFAULT_PET_CONFIG, ...petSection }
    } else {
      cachedConfig = { ...DEFAULT_PET_CONFIG }
    }
  } catch {
    cachedConfig = { ...DEFAULT_PET_CONFIG }
  }

  return cachedConfig!
}

export async function savePetConfig(config: PetConfig): Promise<void> {
  // 合并到完整 config.json 并保存
  const raw = await invoke<string>('load_config')
  const parsed = JSON.parse(raw)
  parsed.pet = config
  await invoke('save_config', { raw: JSON.stringify(parsed, null, 2) })
  cachedConfig = config
}

export function getCachedPetConfig(): PetConfig {
  return cachedConfig ?? DEFAULT_PET_CONFIG
}
```
