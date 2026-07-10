<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { MaskEntryDto, MaskPayload } from '../api/desktop'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  mask?: MaskEntryDto
}>()

const emit = defineEmits<{
  (e: 'save', payload: MaskPayload): void
  (e: 'cancel'): void
}>()

const isEdit = computed(() => !!props.mask)

// Form state
const name = ref('')
const icon = ref('')
const description = ref('')
const mode = ref<'normal' | 'assist'>('normal')
const modelOverride = ref('')
const subagentModel = ref('')
const subagentMaxIterations = ref('')
const allowTags = ref<string[]>([])
const denyTags = ref<string[]>([])
const body = ref('')
const showBodyPreview = ref(false)
const nameError = ref('')

// Tag input helpers
const allowInput = ref('')
const denyInput = ref('')

// Pre-fill when editing; reset for create mode
watch(() => props.mask, (m) => {
  if (m) {
    name.value = m.name
    icon.value = m.icon || ''
    description.value = m.description || ''
    mode.value = (m.mode === 'assist' ? 'assist' : 'normal')
  } else {
    name.value = ''
    icon.value = ''
    description.value = ''
    mode.value = 'normal'
    modelOverride.value = ''
    subagentModel.value = ''
    subagentMaxIterations.value = ''
    allowTags.value = []
    denyTags.value = []
    body.value = ''
    showBodyPreview.value = false
    nameError.value = ''
    allowInput.value = ''
    denyInput.value = ''
  }
}, { immediate: true })

function addAllowTag(): void {
  const trimmed = allowInput.value.trim()
  if (!trimmed || allowTags.value.includes(trimmed)) {
    allowInput.value = ''
    return
  }
  allowTags.value.push(trimmed)
  allowInput.value = ''
}

function addDenyTag(): void {
  const trimmed = denyInput.value.trim()
  if (!trimmed || denyTags.value.includes(trimmed)) {
    denyInput.value = ''
    return
  }
  denyTags.value.push(trimmed)
  denyInput.value = ''
}

function removeAllowTag(tag: string): void {
  allowTags.value = allowTags.value.filter(t => t !== tag)
}

function removeDenyTag(tag: string): void {
  denyTags.value = denyTags.value.filter(t => t !== tag)
}

function handleAllowKeydown(e: KeyboardEvent): void {
  if (e.key === 'Enter') {
    e.preventDefault()
    addAllowTag()
  }
}

function handleDenyKeydown(e: KeyboardEvent): void {
  if (e.key === 'Enter') {
    e.preventDefault()
    addDenyTag()
  }
}

function validate(): boolean {
  if (!name.value.trim()) {
    nameError.value = t('mask.nameRequired')
    return false
  }
  nameError.value = ''
  return true
}

function handleSave(): void {
  if (!validate()) return

  const maxIter = subagentMaxIterations.value
    ? parseInt(subagentMaxIterations.value, 10) || null
    : null

  const payload: MaskPayload = {
    id: isEdit.value ? undefined : null,
    name: name.value.trim(),
    icon: icon.value || null,
    description: description.value || null,
    mode: mode.value,
    model: modelOverride.value || null,
    subagentDefaults: {
      model: subagentModel.value || null,
      max_iterations: maxIter,
    },
    toolLimits: {
      allow: allowTags.value,
      deny: denyTags.value,
    },
    body: body.value || null,
  }
  emit('save', payload)
}

function handleCancel(): void {
  emit('cancel')
}
</script>

<template>
  <div class="bg-white border border-gray-200 rounded-lg p-6 max-w-2xl">
    <h2 class="text-lg font-semibold text-gray-800 mb-4">
      {{ isEdit ? t('mask.editorTitleEdit') : t('mask.editorTitleCreate') }}
    </h2>

    <!-- Name (required) -->
    <div class="mb-4">
      <label class="block text-sm font-medium text-gray-700 mb-1">
        {{ t('mask.name') }} <span class="text-red-500">*</span>
      </label>
      <input
        v-model.trim="name"
        type="text"
        class="w-full border rounded-lg px-3 py-2 text-sm outline-none transition-colors"
        :class="nameError ? 'border-red-400' : 'border-gray-300 focus:border-pink-500 focus:ring-2 focus:ring-pink-500'"
        :placeholder="t('mask.namePlaceholder')"
      />
      <p v-if="nameError" class="text-red-500 text-xs mt-1">{{ nameError }}</p>
    </div>

    <!-- Icon (optional) -->
    <div class="mb-4">
      <label class="block text-sm font-medium text-gray-700 mb-1">{{ t('mask.icon') }}</label>
      <input
        v-model="icon"
        type="text"
        class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm outline-none transition-colors focus:border-pink-500 focus:ring-2 focus:ring-pink-500"
        :placeholder="t('mask.iconPlaceholder')"
        maxlength="10"
      />
    </div>

    <!-- Description (optional) -->
    <div class="mb-4">
      <label class="block text-sm font-medium text-gray-700 mb-1">{{ t('mask.description') }}</label>
      <input
        v-model="description"
        type="text"
        class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm outline-none transition-colors focus:border-pink-500 focus:ring-2 focus:ring-pink-500"
        :placeholder="t('mask.descPlaceholder')"
      />
    </div>

    <!-- Mode (radio: Normal / Assist) -->
    <div class="mb-4">
      <label class="block text-sm font-medium text-gray-700 mb-2">{{ t('mask.mode.label') }}</label>
      <div class="flex gap-6">
        <label class="inline-flex items-center gap-2 cursor-pointer">
          <input
            v-model="mode"
            type="radio"
            value="normal"
            class="text-pink-500 focus:ring-pink-500"
          />
          <span class="text-sm text-gray-700">{{ t('mask.modeNormal') }}</span>
        </label>
        <label class="inline-flex items-center gap-2 cursor-pointer">
          <input
            v-model="mode"
            type="radio"
            value="assist"
            class="text-pink-500 focus:ring-pink-500"
          />
          <span class="text-sm text-gray-700">{{ t('mask.modeAssist') }}</span>
        </label>
      </div>
    </div>

    <!-- Model override (optional) -->
    <div class="mb-4">
      <label class="block text-sm font-medium text-gray-700 mb-1">{{ t('mask.modelOverride') }}</label>
      <input
        v-model="modelOverride"
        type="text"
        class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm outline-none transition-colors focus:border-pink-500 focus:ring-2 focus:ring-pink-500"
        :placeholder="t('mask.modelOverridePlaceholder')"
      />
    </div>

    <!-- Subagent settings -->
    <div class="mb-4 p-3 bg-gray-50 rounded-lg border border-gray-200">
      <h3 class="text-sm font-medium text-gray-700 mb-2">{{ t('mask.subagentSettings') }}</h3>
      <div class="grid grid-cols-2 gap-3">
        <div>
          <label class="block text-xs text-gray-500 mb-1">{{ t('mask.subagentModel') }}</label>
          <input
            v-model="subagentModel"
            type="text"
            class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm outline-none transition-colors focus:border-pink-500 focus:ring-2 focus:ring-pink-500"
            :placeholder="t('mask.optional')"
          />
        </div>
        <div>
          <label class="block text-xs text-gray-500 mb-1">{{ t('mask.subagentMaxIterations') }}</label>
          <input
            v-model="subagentMaxIterations"
            type="number"
            min="1"
            class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm outline-none transition-colors focus:border-pink-500 focus:ring-2 focus:ring-pink-500"
            :placeholder="t('mask.optional')"
          />
        </div>
      </div>
    </div>

    <!-- Tool allow tags -->
    <div class="mb-4">
      <label class="block text-sm font-medium text-gray-700 mb-1">{{ t('mask.toolAllow') }}</label>
      <div v-if="allowTags.length" class="flex flex-wrap gap-1.5 mb-1.5">
        <span
          v-for="tag in allowTags"
          :key="tag"
          class="inline-flex items-center gap-1 px-2 py-0.5 bg-green-100 text-green-700 rounded-full text-xs"
        >
          {{ tag }}
          <button
            type="button"
            class="hover:text-green-900 leading-none text-sm"
            @click="removeAllowTag(tag)"
          >
            &times;
          </button>
        </span>
      </div>
      <input
        v-model="allowInput"
        type="text"
        class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm outline-none transition-colors focus:border-pink-500 focus:ring-2 focus:ring-pink-500"
        :placeholder="t('mask.toolPlaceholder')"
        @keydown="handleAllowKeydown"
      />
    </div>

    <!-- Tool deny tags -->
    <div class="mb-4">
      <label class="block text-sm font-medium text-gray-700 mb-1">{{ t('mask.toolDeny') }}</label>
      <div v-if="denyTags.length" class="flex flex-wrap gap-1.5 mb-1.5">
        <span
          v-for="tag in denyTags"
          :key="tag"
          class="inline-flex items-center gap-1 px-2 py-0.5 bg-red-100 text-red-700 rounded-full text-xs"
        >
          {{ tag }}
          <button
            type="button"
            class="hover:text-red-900 leading-none text-sm"
            @click="removeDenyTag(tag)"
          >
            &times;
          </button>
        </span>
      </div>
      <input
        v-model="denyInput"
        type="text"
        class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm outline-none transition-colors focus:border-pink-500 focus:ring-2 focus:ring-pink-500"
        :placeholder="t('mask.toolPlaceholder')"
        @keydown="handleDenyKeydown"
      />
    </div>

    <!-- Body (textarea with preview toggle) -->
    <div class="mb-6">
      <div class="flex items-center justify-between mb-1">
        <label class="block text-sm font-medium text-gray-700">{{ t('mask.body') }}</label>
        <button
          type="button"
          class="text-xs text-pink-600 hover:text-pink-700 transition-colors"
          @click="showBodyPreview = !showBodyPreview"
        >
          {{ showBodyPreview ? t('mask.edit') : t('mask.preview') }}
        </button>
      </div>
      <textarea
        v-if="!showBodyPreview"
        v-model="body"
        class="w-full border border-gray-300 rounded-lg px-3 py-2 text-sm outline-none transition-colors focus:border-pink-500 focus:ring-2 focus:ring-pink-500"
        rows="6"
        :placeholder="t('mask.bodyPlaceholder')"
      ></textarea>
      <div
        v-else
        class="w-full border border-gray-200 rounded-lg px-3 py-2 text-sm bg-gray-50 min-h-[120px] whitespace-pre-wrap text-gray-700"
      >
        {{ body || t('mask.emptyPreview') }}
      </div>
    </div>

    <!-- Action buttons -->
    <div class="flex items-center justify-end gap-3 pt-4 border-t border-gray-100">
      <button
        type="button"
        class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
        @click="handleCancel"
      >
        {{ t('mask.cancel') }}
      </button>
      <button
        type="button"
        class="px-4 py-2 text-sm text-white bg-pink-500 hover:bg-pink-600 rounded-lg transition-colors"
        @click="handleSave"
      >
        {{ isEdit ? t('mask.saveChanges') : t('mask.create') }}
      </button>
    </div>
  </div>
</template>
