export function resolveGaussSceneUrl(path?: string | null): string | undefined {
  const trimmed = path?.trim()
  if (!trimmed) return undefined
  if (/^(?:https?:|file:|data:|blob:)/i.test(trimmed)) return trimmed
  return trimmed.startsWith('/') ? trimmed : `/${trimmed}`
}
