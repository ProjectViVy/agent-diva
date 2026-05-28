import type { VrmMotionInfo } from '../types'

export const VRM_ANIMATIONS_DIR = '/vrm/animations'

const KNOWN_ANIMATIONS: Array<{ id: string; kind: 'idle' | 'oneshot' }> = [
  { id: 'akimbo', kind: 'idle' },
  { id: 'model_pose', kind: 'idle' },
  { id: 'show_full_body', kind: 'idle' },
  { id: 'greeting', kind: 'oneshot' },
  { id: 'peace_sign', kind: 'oneshot' },
  { id: 'play_fingers', kind: 'oneshot' },
  { id: 'scratch_head', kind: 'oneshot' },
  { id: 'stretch', kind: 'oneshot' },
]

function toDisplayName(id: string): string {
  const names: Record<string, string> = {
    greeting: '问候',
    akimbo: '叉腰',
    peace_sign: '比耶',
    play_fingers: '活动手指',
    scratch_head: '挠头',
    show_full_body: '全身展示',
    stretch: '伸展',
    model_pose: '模型姿势',
  }
  return names[id] ?? id
}

async function fileExists(path: string): Promise<boolean> {
  try {
    const response = await fetch(path, { method: 'HEAD' })
    return response.ok
  } catch {
    return false
  }
}

export async function scanVRMAnimations(): Promise<VrmMotionInfo[]> {
  try {
    const response = await fetch(`${VRM_ANIMATIONS_DIR}/manifest.json`)
    if (response.ok) {
      const manifest = await response.json() as { animations?: string[] }
      if (Array.isArray(manifest.animations)) {
        return manifest.animations.map((name) => {
          const id = name.replace(/\.vrma$/i, '')
          return {
            id,
            name: toDisplayName(id),
            kind: KNOWN_ANIMATIONS.find((motion) => motion.id === id)?.kind,
            path: `${VRM_ANIMATIONS_DIR}/${name}`,
          }
        })
      }
    }
  } catch {
    // Optional manifest is absent in the current bundle.
  }

  const results: VrmMotionInfo[] = []
  await Promise.allSettled(
    KNOWN_ANIMATIONS.map(async (motion) => {
      const path = `${VRM_ANIMATIONS_DIR}/${motion.id}.vrma`
      if (await fileExists(path)) {
        results.push({
          id: motion.id,
          name: toDisplayName(motion.id),
          kind: motion.kind,
          path,
        })
      }
    }),
  )
  return results
}

export function buildKnownMotionInfo(): VrmMotionInfo[] {
  return KNOWN_ANIMATIONS.map((motion) => ({
    id: motion.id,
    name: toDisplayName(motion.id),
    kind: motion.kind,
    path: `${VRM_ANIMATIONS_DIR}/${motion.id}.vrma`,
  }))
}
