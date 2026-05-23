<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { Cat, Clipboard, Mic, Play, RefreshCw, Square, Trash2, Upload, Volume2 } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import { usePetConfig, usePetConfigSaveState } from '../../features/diva-pet/services/pet-config'
import {
  deleteVoiceFile,
  getAsrProviderDefaults,
  importVoiceFile,
  loadVoiceAssets,
  readVoiceFile,
  tauriVoiceFileReader,
  transcribeAudio,
  type VoiceOption,
} from '../../features/diva-pet/voice/services/voice-api'
import { addVoiceLogEvent, clearVoiceLogEvents, useVoiceLog } from '../../features/diva-pet/voice/services/voice-log'
import { getTtsApiKey, type PetConfig } from '../../features/diva-pet/types'
import { isCloudAsrProvider, resolveAsrConfigDefaults } from '../../features/diva-pet/voice/services/asr-service'
import { ttsService, type TTSVoiceConfig } from '../../features/diva-pet/voice/services/tts-service'

const { t } = useI18n()
const { config, updateConfig } = usePetConfig()
const { isSaving, lastSaveError, lastSavedAt } = usePetConfigSaveState()
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

const isCloudAsrSupported = computed(() => {
  if (typeof window === 'undefined') return false
  return typeof navigator !== 'undefined'
    && !!navigator.mediaDevices?.getUserMedia
    && typeof MediaRecorder !== 'undefined'
})

const isSelectedAsrProviderSupported = computed(() => {
  return config.value.asrProvider === 'web_speech'
    ? isWebSpeechSupported.value
    : isCloudAsrSupported.value
})

const selectedVoice = computed(() => {
  return voiceOptions.value.find((voice) => voice.relativePath === config.value.ttsReferenceVoice) ?? null
})

const currentAsrProviderLabel = computed(() => config.value.asrProvider === 'siliconflow' ? 'SiliconFlow' : 'Web Speech')
const isCloudAsrSelected = computed(() => isCloudAsrProvider(config.value.asrProvider))

const currentTtsConfig = computed<TTSVoiceConfig>(() => ({
  enabled: config.value.ttsEnabled,
  provider: config.value.ttsProvider,
  apiKey: getTtsApiKey(config.value),
  baseUrl: config.value.ttsBaseUrl,
  model: config.value.ttsModel,
  voiceId: config.value.ttsVoiceId,
  referenceVoice: config.value.ttsReferenceVoice,
  referenceText: config.value.ttsReferenceText,
  speed: config.value.ttsSpeed,
  volume: config.value.ttsVolume,
}))

const MINIMAX_VOICE_PRESETS = [
  { id: 'male-qn-qingse', label: 'male-qn-qingse' },
  { id: 'audiobook_male_1', label: 'audiobook_male_1' },
]

const SILICONFLOW_VOICE_PRESETS: Record<string, { id: string; label: string }[]> = {
  'fnlp/MOSS-TTSD-v0.5': [
    { id: 'fnlp/MOSS-TTSD-v0.5:alex', label: 'alex' },
    { id: 'fnlp/MOSS-TTSD-v0.5:anna', label: 'anna' },
    { id: 'fnlp/MOSS-TTSD-v0.5:bella', label: 'bella' },
    { id: 'fnlp/MOSS-TTSD-v0.5:benjamin', label: 'benjamin' },
    { id: 'fnlp/MOSS-TTSD-v0.5:charles', label: 'charles' },
    { id: 'fnlp/MOSS-TTSD-v0.5:claire', label: 'claire' },
    { id: 'fnlp/MOSS-TTSD-v0.5:david', label: 'david' },
    { id: 'fnlp/MOSS-TTSD-v0.5:diana', label: 'diana' },
  ],
  'FunAudioLLM/CosyVoice2-0.5B': [
    { id: 'FunAudioLLM/CosyVoice2-0.5B:alex', label: 'alex' },
    { id: 'FunAudioLLM/CosyVoice2-0.5B:anna', label: 'anna' },
    { id: 'FunAudioLLM/CosyVoice2-0.5B:bella', label: 'bella' },
    { id: 'FunAudioLLM/CosyVoice2-0.5B:benjamin', label: 'benjamin' },
    { id: 'FunAudioLLM/CosyVoice2-0.5B:charles', label: 'charles' },
    { id: 'FunAudioLLM/CosyVoice2-0.5B:claire', label: 'claire' },
    { id: 'FunAudioLLM/CosyVoice2-0.5B:david', label: 'david' },
    { id: 'FunAudioLLM/CosyVoice2-0.5B:diana', label: 'diana' },
  ],
}

const siliconflowVoicePresets = computed(() => {
  const model = config.value.ttsModel?.trim()
  if (model && SILICONFLOW_VOICE_PRESETS[model]) {
    return SILICONFLOW_VOICE_PRESETS[model]
  }
  // 默认 MOSS-TTSD
  return SILICONFLOW_VOICE_PRESETS['fnlp/MOSS-TTSD-v0.5']
})

const isMiniMaxProvider = computed(() => config.value.ttsProvider === 'minimax')
const isSiliconFlowProvider = computed(() => config.value.ttsProvider === 'siliconflow')
const isOpenAIProvider = computed(() => config.value.ttsProvider === 'openai')

const TTS_MODEL_PRESETS: Record<string, string[]> = {
  openai: ['tts-1', 'tts-1-hd'],
  siliconflow: ['fnlp/MOSS-TTSD-v0.5', 'FunAudioLLM/CosyVoice2-0.5B'],
  minimax: ['speech-2.8-hd', 'speech-2.8-turbo', 'speech-2.6-hd', 'speech-2.6-turbo'],
}
const ttsModelPresets = computed(() => TTS_MODEL_PRESETS[config.value.ttsProvider] ?? [])
const ttsModelPlaceholder = computed(() => providerDefaults(config.value.ttsProvider).model || 'tts-1')

const saveStateText = computed(() => {
  if (isSaving.value) return '保存中...'
  if (lastSaveError.value) return '保存失败'
  if (lastSavedAt.value) return '已保存'
  return '尚未保存'
})

const saveStateClass = computed(() => {
  if (lastSaveError.value) return 'text-red-600'
  if (isSaving.value) return 'text-amber-600'
  return 'text-emerald-600'
})

function providerDefaults(provider: PetConfig['ttsProvider']) {
  if (provider === 'openai') return { baseUrl: 'https://api.openai.com/v1', model: 'tts-1' }
  if (provider === 'siliconflow') return { baseUrl: 'https://api.siliconflow.cn/v1', model: 'fnlp/MOSS-TTSD-v0.5', voiceId: 'fnlp/MOSS-TTSD-v0.5:anna' }
  if (provider === 'minimax') return { baseUrl: 'https://api.minimaxi.com', model: 'speech-2.8-hd', voiceId: 'male-qn-qingse' }
  return { baseUrl: '', model: '', voiceId: null }
}

function asrProviderDefaults(provider: PetConfig['asrProvider']) {
  const defaults = getAsrProviderDefaults(provider)
  return {
    baseUrl: defaults.baseUrl,
    model: defaults.model,
  }
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

function patchAsrConfig(patch: Partial<PetConfig>): void {
  updateConfig(patch)
  settingsError.value = null
}

function patchVoiceConfig(patch: Partial<PetConfig>): void {
  updateConfig(patch)
  settingsError.value = null
}

function onAsrProviderChange(provider: PetConfig['asrProvider']): void {
  const defaults = asrProviderDefaults(provider)
  patchAsrConfig({
    asrProvider: provider,
    asrBaseUrl: defaults.baseUrl,
    asrModel: defaults.model,
  })
}

function onProviderChange(provider: PetConfig['ttsProvider']): void {
  const defaults = providerDefaults(provider)
  patchVoiceConfig({
    ttsProvider: provider,
    ttsBaseUrl: defaults.baseUrl,
    ttsModel: defaults.model || null,
    ttsVoiceId: defaults.voiceId ?? null,
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

    if (importedVoicePath && isCloudAsrSelected.value && config.value.asrApiKey) {
      const voiceFile = await readVoiceFile(importedVoicePath)
      const resolvedAsr = resolveAsrConfigDefaults(config.value.asrProvider, {
        baseUrl: config.value.asrBaseUrl,
        model: config.value.asrModel,
      })
      const transcribedText = await transcribeAudio({
        base64Data: voiceFile.base64Data,
        fileName: voiceFile.fileName,
        apiKey: config.value.asrApiKey,
        provider: config.value.asrProvider,
        baseUrl: resolvedAsr.baseUrl,
        model: resolvedAsr.model,
        language: config.value.asrLanguage,
        contentType: voiceFile.contentType,
      })
      referenceText = transcribedText || referenceText
    }

    voiceOptions.value = importedAssets.voiceOptions
    updateConfig({
      ttsReferenceVoice: importedVoicePath,
      ttsReferenceText: referenceText,
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
    if (!isCloudAsrSelected.value) {
      throw new Error('开发测试面板的音频转写仅支持云端 ASR Provider。')
    }
    if (!config.value.asrApiKey) {
      throw new Error('当前尚未配置可用于转写的 ASR API Key。')
    }

    const base64Data = await fileToBase64(file)
    const resolvedAsr = resolveAsrConfigDefaults(config.value.asrProvider, {
      baseUrl: config.value.asrBaseUrl,
      model: config.value.asrModel,
    })
    const transcription = await transcribeAudio({
      base64Data,
      fileName: file.name,
      apiKey: config.value.asrApiKey,
      provider: config.value.asrProvider,
      baseUrl: resolvedAsr.baseUrl,
      model: resolvedAsr.model,
      language: config.value.asrLanguage,
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
    <div class="mb-4 rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm">
      <span class="font-medium text-gray-600">配置保存状态：</span>
      <span :class="saveStateClass">{{ saveStateText }}</span>
      <span v-if="lastSavedAt && !isSaving && !lastSaveError" class="ml-2 text-gray-400">{{ new Date(lastSavedAt).toLocaleTimeString() }}</span>
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
          <div class="mt-0.5 text-sm" :class="isSelectedAsrProviderSupported ? 'text-gray-500' : 'text-amber-600'">
            {{ isSelectedAsrProviderSupported ? `${currentAsrProviderLabel} 可用` : `当前环境不支持 ${currentAsrProviderLabel}` }}
          </div>
        </div>
        <button
          class="toggle"
          :class="config.asrEnabled ? 'toggle-on' : 'toggle-off'"
          :disabled="!isSelectedAsrProviderSupported"
          @click="patchAsrConfig({ asrEnabled: !config.asrEnabled })"
        >
          <span :class="config.asrEnabled ? 'translate-x-6' : 'translate-x-1'" />
        </button>
      </div>
      <div class="mt-4 grid grid-cols-1 gap-4 md:grid-cols-2">
        <label class="field-label">
          Provider
          <select class="field-input" :value="config.asrProvider" @change="onAsrProviderChange(($event.target as HTMLSelectElement).value as PetConfig['asrProvider'])">
            <option value="web_speech">Web Speech</option>
            <option value="siliconflow">SiliconFlow</option>
          </select>
        </label>
        <label class="field-label">
          识别语言
          <input class="field-input" :value="config.asrLanguage" @change="patchAsrConfig({ asrLanguage: ($event.target as HTMLInputElement).value || 'zh-CN' })" />
        </label>
      </div>
      <div v-if="config.asrProvider === 'web_speech'" class="mt-3 rounded-lg border border-sky-200 bg-sky-50 px-3 py-2 text-sm text-sky-700">
        Web Speech 依赖系统/浏览器语音识别能力，不需要单独配置云端凭证。
      </div>
      <div v-else class="mt-4 grid grid-cols-1 gap-4 md:grid-cols-2">
        <label class="field-label md:col-span-2">
          Base URL
          <input class="field-input" :value="config.asrBaseUrl" placeholder="https://api.siliconflow.cn/v1" @change="patchAsrConfig({ asrBaseUrl: ($event.target as HTMLInputElement).value })" />
        </label>
        <label class="field-label md:col-span-2">
          API Key
          <input class="field-input" type="password" :value="config.asrApiKey ?? ''" autocomplete="off" @change="patchAsrConfig({ asrApiKey: ($event.target as HTMLInputElement).value || null })" />
        </label>
        <label class="field-label md:col-span-2">
          模型
          <input class="field-input" :value="config.asrModel ?? ''" placeholder="FunAudioLLM/SenseVoiceSmall" @change="patchAsrConfig({ asrModel: ($event.target as HTMLInputElement).value || null })" />
        </label>
      </div>
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
        <button class="toggle" :class="config.ttsEnabled ? 'toggle-on' : 'toggle-off'" @click="patchVoiceConfig({ ttsEnabled: !config.ttsEnabled })">
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
            <option value="minimax">MiniMax</option>
          </select>
        </label>
        <label class="field-label">
          模型
          <select class="field-input" :value="config.ttsModel ?? ''" @change="patchVoiceConfig({ ttsModel: ($event.target as HTMLSelectElement).value || null })">
            <option value="">{{ ttsModelPlaceholder }}（默认）</option>
            <option v-for="model in ttsModelPresets" :key="model" :value="model">{{ model }}</option>
          </select>
        </label>
        <label v-if="isMiniMaxProvider" class="field-label">
          音色 ID
          <select class="field-input" :value="config.ttsVoiceId ?? ''" @change="patchVoiceConfig({ ttsVoiceId: ($event.target as HTMLSelectElement).value || null })">
            <option value="">默认</option>
            <option v-for="voice in MINIMAX_VOICE_PRESETS" :key="voice.id" :value="voice.id">{{ voice.label }}</option>
          </select>
        </label>
        <label v-if="isSiliconFlowProvider" class="field-label">
          音色
          <select class="field-input" :value="config.ttsVoiceId ?? ''" @change="patchVoiceConfig({ ttsVoiceId: ($event.target as HTMLSelectElement).value || null })">
            <option value="">默认</option>
            <option v-for="voice in siliconflowVoicePresets" :key="voice.id" :value="voice.id">{{ voice.label }}</option>
          </select>
        </label>
        <label class="field-label md:col-span-2">
          Base URL
          <input class="field-input" :value="config.ttsBaseUrl" placeholder="https://api.siliconflow.cn/v1" @change="patchVoiceConfig({ ttsBaseUrl: ($event.target as HTMLInputElement).value })" />
        </label>
        <label class="field-label md:col-span-2">
          API Key
          <template v-if="isMiniMaxProvider">
            <input class="field-input" type="password" :value="config.ttsMinimaxApiKey ?? ''" autocomplete="off" placeholder="请输入 MiniMax API Key" @change="patchVoiceConfig({ ttsMinimaxApiKey: ($event.target as HTMLInputElement).value || null })" />
          </template>
          <template v-else-if="isSiliconFlowProvider">
            <input class="field-input" type="password" :value="config.ttsSiliconflowApiKey ?? ''" autocomplete="off" placeholder="请输入 SiliconFlow API Key" @change="patchVoiceConfig({ ttsSiliconflowApiKey: ($event.target as HTMLInputElement).value || null })" />
          </template>
          <template v-else-if="isOpenAIProvider">
            <input class="field-input" type="password" :value="config.ttsOpenaiApiKey ?? ''" autocomplete="off" placeholder="请输入 OpenAI API Key" @change="patchVoiceConfig({ ttsOpenaiApiKey: ($event.target as HTMLInputElement).value || null })" />
          </template>
          <input v-else class="field-input" type="password" :value="''" autocomplete="off" disabled placeholder="Browser TTS 不需要 API Key" />
        </label>
        <label class="field-label">
          语速 {{ config.ttsSpeed.toFixed(2) }}
          <input class="w-full" type="range" min="0.5" max="2" step="0.05" :value="config.ttsSpeed" @input="patchVoiceConfig({ ttsSpeed: Number(($event.target as HTMLInputElement).value) })" />
        </label>
        <label class="field-label">
          音量 {{ config.ttsVolume.toFixed(2) }}
          <input class="w-full" type="range" min="0" max="1" step="0.05" :value="config.ttsVolume" @input="patchVoiceConfig({ ttsVolume: Number(($event.target as HTMLInputElement).value) })" />
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
        <template v-if="isMiniMaxProvider">
          <div class="rounded-lg border border-sky-200 bg-sky-50 px-3 py-2 text-sm text-sky-700">
            MiniMax 首版仅支持系统音色，不支持参考音色导入或复刻音色。
          </div>
        </template>
        <template v-else>
          <input ref="fileInput" class="hidden" type="file" accept="audio/*" @change="(event) => void onImportVoice(event)" />
          <select class="field-input" :value="config.ttsReferenceVoice ?? ''" @change="patchVoiceConfig({ ttsReferenceVoice: ($event.target as HTMLSelectElement).value || null })">
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
              @change="patchVoiceConfig({ ttsReferenceText: ($event.target as HTMLTextAreaElement).value || null })"
            />
          </label>
        </template>
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
      <div v-if="!isCloudAsrSelected" class="mb-4 rounded-lg border border-sky-200 bg-sky-50 px-3 py-2 text-sm text-sky-700">
        当前 ASR Provider 为 Web Speech，开发测试面板的文件转写能力仅在云端 ASR Provider 下可用。
      </div>

      <div class="dev-test-actions">
        <button class="dev-test-btn" :disabled="isRunningVoiceDevTest || !isCloudAsrSelected" @click="devAudioInput?.click()">
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
