const DEFAULT_VRM_MODEL_FILE = 'Alice.vrm'
const VRM_MODELS_BASE_PATH = '/vrm/models/'

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
