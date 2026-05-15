/**
 * VRMA animation file scanner.
 *
 * Scans the /public/vrm/animations/ directory for VRMA files and
 * produces a VrmMotionInfo[] list for the idle animation system.
 *
 * Strategy:
 *  1. Try fetching a static manifest.json from /vrm/animations/ (fast, bundled)
 *  2. Fall back to a hard-coded known-animations list
 *  3. Filter out files that do not actually exist via HEAD requests
 *
 * Adapted from super-agent-party's VRMConfig.vrm_motion_list.
 */

import type { VrmMotionInfo } from '../types'

/** Default VRMA animation directory relative to public/ */
export const VRM_ANIMATIONS_DIR = '/vrm/animations'

/** Standard motion names known from super-agent-party's reference set */
const KNOWN_ANIMATIONS: string[] = [
  'greeting',
  'akimbo',
  'peace_sign',
  'play_fingers',
  'scratch_head',
  'shoot',
  'show_full_body',
  'spin',
  'squat',
  'stretch',
  'model_pose',
]

/**
 * Convert a filename (without extension) to a human-readable display name.
 * Falls back to the raw id if no mapping exists.
 */
function toDisplayName(id: string): string {
  const NAME_MAP: Record<string, string> = {
    greeting: '问候',
    akimbo: '叉腰',
    peace_sign: '和平手势',
    play_fingers: '玩手指',
    scratch_head: '挠头',
    shoot: '射击',
    show_full_body: '全身展示',
    spin: '旋转',
    squat: '蹲下',
    stretch: '伸展',
    model_pose: '模型姿势',
  }
  return NAME_MAP[id] ?? id
}

/**
 * Check if a given URL returns a successful response via HEAD.
 */
async function fileExists(path: string): Promise<boolean> {
  try {
    const response = await fetch(path, { method: 'HEAD' })
    return response.ok
  } catch {
    return false
  }
}

/**
 * Scan VRMA animation files from the public directory.
 *
 * Performs light validation — does NOT parse VRMA binary content.
 * Callers should further validate animations via IdleAnimationManager.
 *
 * @returns Resolved VrmMotionInfo list (empty array if no animations found)
 */
export async function scanVRMAnimations(): Promise<VrmMotionInfo[]> {
  // Strategy 1: manifest.json
  const manifestPath = `${VRM_ANIMATIONS_DIR}/manifest.json`
  try {
    const response = await fetch(manifestPath)
    if (response.ok) {
      const manifest = await response.json() as { animations?: string[] }
      if (Array.isArray(manifest.animations)) {
        return manifest.animations.map((name) => ({
          id: name.replace(/\.vrma$/i, ''),
          name: toDisplayName(name.replace(/\.vrma$/i, '')),
          path: `${VRM_ANIMATIONS_DIR}/${name}`,
        }))
      }
    }
  } catch {
    // manifest.json not available, fall through
  }

  // Strategy 2: known animations list with existence check
  const results: VrmMotionInfo[] = []
  const checks = KNOWN_ANIMATIONS.map(async (animationId) => {
    const path = `${VRM_ANIMATIONS_DIR}/${animationId}.vrma`
    const exists = await fileExists(path)
    if (exists) {
      results.push({
        id: animationId,
        name: toDisplayName(animationId),
        path,
      })
    }
  })

  await Promise.allSettled(checks)

  console.log(
    `[vrm-animation-scanner] Found ${results.length} VRMA animations in ${VRM_ANIMATIONS_DIR}`,
  )
  return results
}

/**
 * Synchronous helper: build a VrmMotionInfo from a known animation ID.
 * Useful for populating initial config without async scanning.
 */
export function buildKnownMotionInfo(): VrmMotionInfo[] {
  return KNOWN_ANIMATIONS.map((id) => ({
    id,
    name: toDisplayName(id),
    path: `${VRM_ANIMATIONS_DIR}/${id}.vrma`,
  }))
}
