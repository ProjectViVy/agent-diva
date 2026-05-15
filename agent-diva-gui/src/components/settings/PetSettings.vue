<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { Cat, Clipboard, Mic, Play, RefreshCw, Square, Trash2, Upload, Volume2 } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import { usePetConfig } from '../../features/diva-pet/services/pet-config'
import {
  deleteVoiceFile,
  importVoiceFile,
  loadVoiceAssets,
  readVoiceFile,
  saveVoiceSelection,
  tauriVoiceFileReader,
  transcribeAudio,
  type VoiceOption,
} from '../../features/diva-pet/voice/services/voice-api'
import { addVoiceLogEvent, clearVoiceLogEvents, useVoiceLog } from '../../features/diva-pet/voice/services/voice-log'
import type { PetConfig } from '../../features/diva-pet/types'
import { ttsService, type TTSVoiceConfig } from '../../features/diva-pet/voice/services/tts-service'

const { t } = useI18n()
const { config, updateConfig } = usePetConfig()
const { recentEvents } = useVoiceLog()

ttsService.setVoiceFileReader(tauriVoiceFileReader)

const voiceOptions = ref<VoiceOption[]>([])

const SCENE_OPTIONS = [
  { id: 'transparent', icon: 'T', label: '透明背景', desc: '仅显示 VRM 模型和阴影' },
  { id: 'home', icon: 'H', label: '室内场景', desc: '默认室内 Gaussian Splat 场景' },
  { id: 'sea', icon: 'S', label: '海边场景', desc: '明亮海边环境' },
  { id: 'space', icon: '*', label: '太空场景', desc: '低干扰深色背景' },
]

const isLoadingVoiceAssets = ref(false)
const fileInput = ref<HTMLInputElement | null>(null)
const devAudioInput = ref<HTMLInputElement | null>(null)
const settingsError = ref<string | null>(null)
const isRunningVoiceDevTest = ref(false)
const devAudioFileName = ref<string | null>(null)
const devTranscription = ref('')

function tt(key: string, fallback: string): string {
  const result = t(key)
  return result === key ? fallback : result
}

const currentModelName = computed(() => config.value.vrmModel || tt('pet.config.noModel', '未选择模型'))

const isWebSpeechSupported = computed(() => {
  if (typeof window === 'undefined') return false
  return Boolean(window.SpeechRecognition || window.webkitSpeechRecognition)
})

const selectedVoice = computed(() => {
  return voiceOptions.value.find((voice) => voice.relativePath === config.value.ttsReferenceVoice) ?? null
})

const currentTtsConfig = computed<TTSVoiceConfig>(() => ({
  enabled: config.value.ttsEnabled,
  provider: config.value.ttsProvider,
  apiKey: config.value.ttsApiKey,
  baseUrl: config.value.ttsBaseUrl,
  model: config.value.ttsModel,
  referenceVoice: config.value.ttsReferenceVoice,
  referenceText: config.value.ttsReferenceText,
  speed: config.value.ttsSpeed,
  volume: config.value.ttsVolume,
}))

function providerDefaults(provider: PetConfig['ttsProvider']) {
  if (provider === 'openai') return { baseUrl: 'https://api.openai.com/v1', model: 'tts-1' }
  if (provider === 'siliconflow') return { baseUrl: 'https://api.siliconflow.cn/v1', model: 'FunAudioLLM/CosyVoice2-0.5B' }
  return { baseUrl: '', model: '' }
}

async function refreshVoiceAssets(): Promise<void> {
  isLoadingVoiceAssets.value = true
  settingsError.value = null
  try {
    const assets = await loadVoiceAssets()
    voiceOptions.value = assets.voiceOptions
  } catch (error) {
    settingsError.value = String(error)
    addVoiceLogEvent({
      level: 'error',
      source: 'settings',
      message: '加载音色资源失败',
      detail: { error: String(error) },
    })
  } finally {
    isLoadingVoiceAssets.value = false
  }
}

function patchConfig(patch: Partial<PetConfig>): void {
  updateConfig(patch)
}

async function patchVoiceConfig(patch: Partial<PetConfig>): Promise<void> {
  const next = { ...config.value, ...patch }
  updateConfig(patch)
  try {
    await saveVoiceSelection({
      enabled: next.ttsEnabled,
      provider: next.ttsProvider,
      apiKey: next.ttsApiKey,
      baseUrl: next.ttsBaseUrl || null,
      model: next.ttsModel,
      referenceVoice: next.ttsReferenceVoice,
      referenceText: next.ttsReferenceText,
      speed: next.ttsSpeed,
      volume: next.ttsVolume,
    })
    await refreshVoiceAssets()
  } catch (error) {
    settingsError.value = String(error)
    addVoiceLogEvent({
      level: 'error',
      source: 'settings',
      message: '保存 TTS 设置失败',
      detail: { error: String(error) },
    })
  }
}

function onProviderChange(provider: PetConfig['ttsProvider']): void {
  const defaults = providerDefaults(provider)
  void patchVoiceConfig({
    ttsProvider: provider,
    ttsBaseUrl: config.value.ttsBaseUrl || defaults.baseUrl,
    ttsModel: config.value.ttsModel || defaults.model || null,
  })
}

async function onImportVoice(event: Event): Promise<void> {
  const target = event.target as HTMLInputElement
  const file = target.files?.[0]
  target.value = ''
  if (!file) return

  isLoadingVoiceAssets.value = true
  settingsError.value = null
  try {
    const importedAssets = await importVoiceFile(file)
    const importedVoicePath = importedAssets.activeVoice.referenceVoice
    let referenceText = importedAssets.activeVoice.referenceText

    if (importedVoicePath && importedAssets.activeVoice.apiKey) {
      const voiceFile = await readVoiceFile(importedVoicePath)
      const transcribedText = await transcribeAudio({
        base64Data: voiceFile.base64Data,
        fileName: voiceFile.fileName,
        apiKey: importedAssets.activeVoice.apiKey,
        baseUrl: importedAssets.activeVoice.baseUrl,
        contentType: voiceFile.contentType,
      })
      referenceText = transcribedText || referenceText
    }

    const assets = await saveVoiceSelection({
      enabled: importedAssets.activeVoice.enabled,
      provider: importedAssets.activeVoice.provider,
      apiKey: importedAssets.activeVoice.apiKey,
      baseUrl: importedAssets.activeVoice.baseUrl,
      model: importedAssets.activeVoice.model,
      referenceVoice: importedVoicePath,
      referenceText,
      speed: importedAssets.activeVoice.speed,
      volume: importedAssets.activeVoice.volume,
    })

    voiceOptions.value = assets.voiceOptions
    updateConfig({
      ttsProvider: assets.activeVoice.provider === 'siliconflow' ? 'siliconflow' : config.value.ttsProvider,
      ttsReferenceVoice: assets.activeVoice.referenceVoice,
      ttsReferenceText: assets.activeVoice.referenceText,
    })
    addVoiceLogEvent({
      level: 'info',
      source: 'settings',
      message: '已导入参考音色',
      detail: { fileName: file.name },
    })
  } catch (error) {
    settingsError.value = String(error)
    addVoiceLogEvent({
      level: 'error',
      source: 'settings',
      message: '导入参考音色失败',
      detail: { fileName: file.name, error: String(error) },
    })
  } finally {
    isLoadingVoiceAssets.value = false
  }
}

async function onRunVoiceDevTest(event: Event): Promise<void> {
  const target = event.target as HTMLInputElement
  const file = target.files?.[0]
  target.value = ''
  if (!file) return

  devAudioFileName.value = file.name
  devTranscription.value = ''
  settingsError.value = null
  isRunningVoiceDevTest.value = true

  try {
    if (!config.value.ttsApiKey) {
      throw new Error('当前尚未配置可用于转写的 API Key。')
    }

    const base64Data = await fileToBase64(file)
    const transcription = await transcribeAudio({
      base64Data,
      fileName: file.name,
      apiKey: config.value.ttsApiKey,
      baseUrl: config.value.ttsBaseUrl,
      contentType: file.type || 'application/octet-stream',
    })

    devTranscription.value = transcription

    if (!transcription.trim()) {
      throw new Error('音频已上传，但未获得可用转写结果。')
    }

    await ttsService.speakText(transcription, currentTtsConfig.value)
    addVoiceLogEvent({
      level: 'info',
      source: 'settings',
      message: '开发测试面板播报完成',
      detail: { fileName: file.name, textPreview: transcription.slice(0, 80) },
    })
  } catch (error) {
    settingsError.value = error instanceof Error ? error.message : String(error)
    addVoiceLogEvent({
      level: 'error',
      source: 'settings',
      message: '开发测试面板执行失败',
      detail: { fileName: file.name, error: String(error) },
    })
  } finally {
    isRunningVoiceDevTest.value = false
  }
}

async function replayDevTranscription(): Promise<void> {
  const text = devTranscription.value.trim()
  if (!text) return

  isRunningVoiceDevTest.value = true
  settingsError.value = null
  try {
    await ttsService.speakText(text, currentTtsConfig.value)
  } catch (error) {
    settingsError.value = error instanceof Error ? error.message : String(error)
  } finally {
    isRunningVoiceDevTest.value = false
  }
}

function stopDevPlayback(): void {
  ttsService.stopPlayback()
  isRunningVoiceDevTest.value = false
}

async function deleteSelectedVoice(): Promise<void> {
  if (!selectedVoice.value || selectedVoice.value.source !== 'custom') return

  isLoadingVoiceAssets.value = true
  settingsError.value = null
  try {
    const assets = await deleteVoiceFile(selectedVoice.value.relativePath)
    voiceOptions.value = assets.voiceOptions
    updateConfig({
      ttsReferenceVoice: assets.activeVoice.referenceVoice,
      ttsReferenceText: assets.activeVoice.referenceText,
    })
    addVoiceLogEvent({
      level: 'info',
      source: 'settings',
      message: '已删除参考音色',
      detail: { path: selectedVoice.value.relativePath },
    })
  } catch (error) {
    settingsError.value = String(error)
  } finally {
    isLoadingVoiceAssets.value = false
  }
}

function formatEventTime(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

function eventDetailText(detail: Record<string, string | number | boolean | null> | undefined): string {
  if (!detail) return ''
  return Object.entries(detail)
    .map(([key, value]) => `${key}: ${value ?? '-'}`)
    .join(' | ')
}

async function copyLogs(): Promise<void> {
  const text = recentEvents.value
    .map((event) => `[${formatEventTime(event.at)}] ${event.source}/${event.level} ${event.message} ${eventDetailText(event.detail)}`)
    .join('\n')
  await navigator.clipboard.writeText(text)
}

function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onerror = () => reject(new Error('Failed to read audio file.'))
    reader.onload = () => {
      const result = String(reader.result ?? '')
      resolve(result.includes(',') ? result.split(',')[1] : result)
    }
    reader.readAsDataURL(file)
  })
}

onMounted(() => {
  void refreshVoiceAssets()
})
</script>

<template>
  <div class="p-8 fade-in max-w-3xl mx-auto">
    <div class="text-center mb-8">
      <div class="w-16 h-16 bg-pink-100 text-pink-600 rounded-2xl flex items-center justify-center mx-auto mb-4">
        <Cat :size="32" />
      </div>
      <h2 class="text-2xl font-bold text-gray-800">{{ tt('pet.config.title', 'Diva Pet 设置') }}</h2>
      <p class="text-gray-500 mt-2">{{ tt('pet.config.desc', '配置桌面数字人显示与语音交互') }}</p>
    </div>

    <div v-if="settingsError" class="mb-4 rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm text-amber-700">
      {{ settingsError }}
    </div>

    <section class="settings-card">
      <h3 class="settings-title">基础设置</h3>
      <div class="setting-row">
        <div>
          <div class="font-medium text-gray-800">启用桌宠</div>
          <div class="mt-0.5 text-sm text-gray-500">控制 Diva Pet 入口和桌宠视图</div>
        </div>
        <button class="toggle" :class="config.enabled ? 'toggle-on' : 'toggle-off'" @click="patchConfig({ enabled: !config.enabled })">
          <span :class="config.enabled ? 'translate-x-6' : 'translate-x-1'" />
        </button>
      </div>

      <div class="setting-row mt-3">
        <div>
          <div class="font-medium text-gray-800">当前模型</div>
          <div class="mt-0.5 text-sm text-gray-500">{{ currentModelName }}</div>
        </div>
      </div>
    </section>

    <section class="settings-card">
      <h3 class="settings-title">3D 背景场景</h3>
      <div class="flex flex-col gap-2">
        <label
          v-for="opt in SCENE_OPTIONS"
          :key="opt.id"
          class="scene-option flex cursor-pointer items-center gap-3 rounded-lg border border-gray-200 bg-white p-3 transition-all duration-150"
          :class="{ 'border-pink-300 bg-pink-50': config.selectedGaussSceneId === opt.id }"
        >
          <input
            v-model="config.selectedGaussSceneId"
            type="radio"
            :value="opt.id"
            class="h-4 w-4 accent-pink-500"
          />
          <span class="w-8 text-center text-xl">{{ opt.icon }}</span>
          <div class="flex flex-col gap-0.5">
            <span class="text-sm font-medium text-gray-800">{{ opt.label }}</span>
            <span class="text-xs text-gray-400">{{ opt.desc }}</span>
          </div>
        </label>
      </div>
      <p class="mt-3 text-xs italic text-gray-400">切换立即生效，场景加载约 1-3 秒。</p>
    </section>

    <section class="settings-card">
      <div class="mb-4 flex items-center gap-2">
        <Mic :size="18" class="text-pink-500" />
        <h3 class="settings-title mb-0">ASR 语音输入</h3>
      </div>
      <div class="setting-row">
        <div>
          <div class="font-medium text-gray-800">启用语音输入</div>
          <div class="mt-0.5 text-sm" :class="isWebSpeechSupported ? 'text-gray-500' : 'text-amber-600'">
            {{ isWebSpeechSupported ? 'Web Speech API 可用' : '当前环境不支持 Web Speech API' }}
          </div>
        </div>
        <button
          class="toggle"
          :class="config.asrEnabled ? 'toggle-on' : 'toggle-off'"
          :disabled="!isWebSpeechSupported"
          @click="patchConfig({ asrEnabled: !config.asrEnabled })"
        >
          <span :class="config.asrEnabled ? 'translate-x-6' : 'translate-x-1'" />
        </button>
      </div>
      <label class="field-label mt-4">
        识别语言
        <input class="field-input" :value="config.asrLanguage" @change="patchConfig({ asrLanguage: ($event.target as HTMLInputElement).value || 'zh-CN' })" />
      </label>
    </section>

    <section class="settings-card">
      <div class="mb-4 flex items-center gap-2">
        <Volume2 :size="18" class="text-pink-500" />
        <h3 class="settings-title mb-0">TTS 语音播报</h3>
      </div>
      <div class="setting-row">
        <div>
          <div class="font-medium text-gray-800">启用播报</div>
          <div class="mt-0.5 text-sm text-gray-500">AI 回复后自动播放语音</div>
        </div>
        <button class="toggle" :class="config.ttsEnabled ? 'toggle-on' : 'toggle-off'" @click="void patchVoiceConfig({ ttsEnabled: !config.ttsEnabled })">
          <span :class="config.ttsEnabled ? 'translate-x-6' : 'translate-x-1'" />
        </button>
      </div>

      <div class="mt-4 grid grid-cols-1 gap-4 md:grid-cols-2">
        <label class="field-label">
          Provider
          <select class="field-input" :value="config.ttsProvider" @change="onProviderChange(($event.target as HTMLSelectElement).value as PetConfig['ttsProvider'])">
            <option value="browser">Browser</option>
            <option value="openai">OpenAI</option>
            <option value="siliconflow">SiliconFlow</option>
          </select>
        </label>
        <label class="field-label">
          模型
          <input class="field-input" :value="config.ttsModel ?? ''" placeholder="tts-1" @change="void patchVoiceConfig({ ttsModel: ($event.target as HTMLInputElement).value || null })" />
        </label>
        <label class="field-label md:col-span-2">
          Base URL
          <input class="field-input" :value="config.ttsBaseUrl" placeholder="https://api.siliconflow.cn/v1" @change="void patchVoiceConfig({ ttsBaseUrl: ($event.target as HTMLInputElement).value })" />
        </label>
        <label class="field-label md:col-span-2">
          API Key
          <input class="field-input" type="password" :value="config.ttsApiKey ?? ''" autocomplete="off" @change="void patchVoiceConfig({ ttsApiKey: ($event.target as HTMLInputElement).value || null })" />
        </label>
        <label class="field-label">
          语速 {{ config.ttsSpeed.toFixed(2) }}
          <input class="w-full" type="range" min="0.5" max="2" step="0.05" :value="config.ttsSpeed" @input="void patchVoiceConfig({ ttsSpeed: Number(($event.target as HTMLInputElement).value) })" />
        </label>
        <label class="field-label">
          音量 {{ config.ttsVolume.toFixed(2) }}
          <input class="w-full" type="range" min="0" max="1" step="0.05" :value="config.ttsVolume" @input="void patchVoiceConfig({ ttsVolume: Number(($event.target as HTMLInputElement).value) })" />
        </label>
      </div>

      <div class="mt-5 border-t border-gray-100 pt-4">
        <div class="mb-3 flex items-center justify-between gap-3">
          <h4 class="font-medium text-gray-800">参考音色</h4>
          <div class="flex items-center gap-2">
            <button class="icon-btn" :disabled="isLoadingVoiceAssets" title="刷新" @click="void refreshVoiceAssets()">
              <RefreshCw :size="15" />
            </button>
            <button class="icon-btn" :disabled="isLoadingVoiceAssets" title="导入音色" @click="fileInput?.click()">
              <Upload :size="15" />
            </button>
            <button class="icon-btn danger" :disabled="selectedVoice?.source !== 'custom' || isLoadingVoiceAssets" title="删除音色" @click="void deleteSelectedVoice()">
              <Trash2 :size="15" />
            </button>
          </div>
        </div>
        <input ref="fileInput" class="hidden" type="file" accept="audio/*" @change="(event) => void onImportVoice(event)" />
        <select class="field-input" :value="config.ttsReferenceVoice ?? ''" @change="void patchVoiceConfig({ ttsReferenceVoice: ($event.target as HTMLSelectElement).value || null })">
          <option value="">不使用参考音色</option>
          <option v-for="voice in voiceOptions" :key="voice.id" :value="voice.relativePath">
            {{ voice.label }} | {{ voice.source }}
          </option>
        </select>
        <label class="field-label mt-3">
          参考文本
          <textarea
            class="field-input min-h-20"
            :value="config.ttsReferenceText ?? ''"
            @change="void patchVoiceConfig({ ttsReferenceText: ($event.target as HTMLTextAreaElement).value || null })"
          />
        </label>
      </div>
    </section>

    <section class="settings-card">
      <div class="mb-4 flex items-center gap-2">
        <Mic :size="18" class="text-pink-500" />
        <h3 class="settings-title mb-0">ASR / TTS 开发测试</h3>
      </div>
      <p class="mb-4 text-sm text-gray-500">
        上传一段音频，使用当前配置先转写，再用当前 TTS 配置播报转写结果。
      </p>

      <div class="dev-test-actions">
        <button class="dev-test-btn" :disabled="isRunningVoiceDevTest" @click="devAudioInput?.click()">
          <Upload :size="15" />
          <span>{{ isRunningVoiceDevTest ? '处理中...' : '上传音频并测试' }}</span>
        </button>
        <button class="dev-test-btn" :disabled="!devTranscription.trim() || isRunningVoiceDevTest" @click="void replayDevTranscription()">
          <Play :size="15" />
          <span>重新播报</span>
        </button>
        <button class="dev-test-btn dev-test-btn--danger" :disabled="!isRunningVoiceDevTest" @click="stopDevPlayback">
          <Square :size="14" />
          <span>停止</span>
        </button>
      </div>

      <input ref="devAudioInput" class="hidden" type="file" accept="audio/*" @change="(event) => void onRunVoiceDevTest(event)" />

      <div class="dev-test-body">
        <div class="dev-test-meta">
          <span class="font-medium text-gray-800">最近测试文件：</span>
          {{ devAudioFileName ?? '暂无' }}
        </div>
        <label class="field-label dev-test-transcription">
          转写结果
          <textarea
            v-model="devTranscription"
            class="field-input min-h-24"
            placeholder="上传音频后会在这里显示转写文本，可手动修改后重新播报。"
          />
        </label>
      </div>
    </section>

    <section class="settings-card">
      <div class="mb-4 flex items-center justify-between gap-3">
        <h3 class="settings-title mb-0">ASR/TTS 日志输出</h3>
        <div class="flex items-center gap-2">
          <button class="icon-btn" title="复制日志" @click="void copyLogs()"><Clipboard :size="15" /></button>
          <button class="icon-btn danger" title="清空日志" @click="clearVoiceLogEvents"><Trash2 :size="15" /></button>
        </div>
      </div>
      <div class="log-list">
        <div v-if="recentEvents.length === 0" class="py-4 text-center text-sm text-gray-400">暂无语音事件</div>
        <div v-for="event in recentEvents" :key="event.id" class="log-row">
          <div class="flex items-center gap-2">
            <span class="text-[11px] text-gray-400">{{ formatEventTime(event.at) }}</span>
            <span class="log-chip" :class="event.level">{{ event.source }} | {{ event.level }}</span>
            <span class="text-sm text-gray-700">{{ event.message }}</span>
          </div>
          <div v-if="event.detail" class="mt-1 text-xs text-gray-400">{{ eventDetailText(event.detail) }}</div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.fade-in {
  animation: slideIn 0.3s ease-out;
}

.settings-card {
  @apply mb-6 rounded-xl border border-gray-100 bg-gray-50 p-6;
}

.settings-title {
  @apply mb-4 text-sm font-semibold uppercase tracking-wider text-gray-500;
}

.setting-row {
  @apply flex items-center justify-between gap-4 rounded-lg border border-gray-200 bg-white p-4 shadow-sm;
}

.scene-option:hover {
  @apply border-pink-200 bg-pink-50/30;
}

.toggle {
  @apply relative inline-flex h-6 w-11 flex-shrink-0 items-center rounded-full transition-colors duration-200 disabled:cursor-not-allowed disabled:opacity-50;
}

.toggle span {
  @apply inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform duration-200;
}

.toggle-on {
  @apply bg-pink-500;
}

.toggle-off {
  @apply bg-gray-300;
}

.field-label {
  @apply block text-sm font-medium text-gray-600;
}

.field-input {
  @apply mt-1 w-full rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm text-gray-800 outline-none focus:border-pink-300 focus:ring-2 focus:ring-pink-100;
}

.icon-btn {
  @apply flex h-8 w-8 items-center justify-center rounded-lg border border-gray-200 bg-white text-gray-500 hover:border-pink-200 hover:text-pink-600 disabled:cursor-not-allowed disabled:opacity-40;
}

.icon-btn.danger {
  @apply hover:border-red-200 hover:text-red-600;
}

.dev-test-actions {
  @apply flex flex-wrap gap-2;
}

.dev-test-btn {
  @apply inline-flex min-h-10 items-center justify-center gap-2 rounded-lg border border-gray-200 bg-white px-4 py-2 text-sm text-gray-600 transition-colors hover:border-pink-200 hover:text-pink-600 disabled:cursor-not-allowed disabled:opacity-40;
}

.dev-test-btn--danger {
  @apply hover:border-red-200 hover:text-red-600;
}

.dev-test-body {
  @apply mt-4 grid grid-cols-1 gap-3;
}

.dev-test-meta {
  @apply rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm text-gray-600;
}

.dev-test-transcription {
  @apply mb-0;
}

.log-list {
  @apply max-h-80 overflow-y-auto rounded-lg border border-gray-200 bg-white;
}

.log-row {
  @apply border-b border-gray-100 px-3 py-2 last:border-b-0;
}

.log-chip {
  @apply rounded-full px-2 py-0.5 text-[10px] uppercase;
}

.log-chip.info {
  @apply bg-blue-50 text-blue-600;
}

.log-chip.warn {
  @apply bg-amber-50 text-amber-600;
}

.log-chip.error {
  @apply bg-red-50 text-red-600;
}

@keyframes slideIn {
  from { opacity: 0; transform: translateX(20px); }
  to { opacity: 1; transform: translateX(0); }
}
</style>
