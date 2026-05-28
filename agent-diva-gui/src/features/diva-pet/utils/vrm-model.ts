const DEFAULT_VRM_MODEL_FILE = 'Alice.vrm'
const VRM_MODELS_BASE_PATH = '/vrm/models/'
const CUSTOM_VRM_MODELS_PREFIX = 'vrm/models/custom/'

function normalizeModelValue(model: string | null | undefined): string {
  return (model ?? '').trim().replace(/\\/g, '/')
}

export function resolveVrmModelPath(model: string | null | undefined): string {
  const normalized = normalizeModelValue(model)
  if (!normalized) {
    return `${VRM_MODELS_BASE_PATH}${DEFAULT_VRM_MODEL_FILE}`
  }

  if (normalized.startsWith(VRM_MODELS_BASE_PATH)) {
    return normalized
  }

  if (normalized.startsWith(CUSTOM_VRM_MODELS_PREFIX)) {
    return normalized
  }

  if (normalized.startsWith(VRM_MODELS_BASE_PATH.slice(1))) {
    return `/${normalized}`
  }

  const fileName = /\.vrm$/i.test(normalized) ? normalized : `${normalized}.vrm`
  return `${VRM_MODELS_BASE_PATH}${fileName}`
}

export function toVrmModelId(model: string | null | undefined): string {
  const normalized = normalizeModelValue(model)
  if (!normalized) {
    return ''
  }

  const fileName = normalized.split('/').pop() ?? normalized
  return fileName.replace(/\.vrm$/i, '')
}

export function isCustomVrmModelPath(model: string | null | undefined): boolean {
  const normalized = normalizeModelValue(model)
  return normalized.startsWith(CUSTOM_VRM_MODELS_PREFIX)
}
