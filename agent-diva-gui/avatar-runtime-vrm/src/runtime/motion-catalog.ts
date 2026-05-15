import type { AvatarMotionEntry } from '../protocol'

const BUILTIN_MOTIONS: AvatarMotionEntry[] = [
  {
    id: 'akimbo',
    name: 'Akimbo',
    kind: 'idle',
    source: '/vrm/animations/akimbo.vrma',
  },
  {
    id: 'model_pose',
    name: 'Model Pose',
    kind: 'idle',
    source: '/vrm/animations/model_pose.vrma',
  },
  {
    id: 'show_full_body',
    name: 'Show Full Body',
    kind: 'idle',
    source: '/vrm/animations/show_full_body.vrma',
  },
  {
    id: 'greeting',
    name: 'Greeting',
    kind: 'oneshot',
    source: '/vrm/animations/greeting.vrma',
  },
  {
    id: 'peace_sign',
    name: 'Peace Sign',
    kind: 'oneshot',
    source: '/vrm/animations/peace_sign.vrma',
  },
  {
    id: 'play_fingers',
    name: 'Play Fingers',
    kind: 'oneshot',
    source: '/vrm/animations/play_fingers.vrma',
  },
  {
    id: 'scratch_head',
    name: 'Scratch Head',
    kind: 'oneshot',
    source: '/vrm/animations/scratch_head.vrma',
  },
  {
    id: 'stretch',
    name: 'Stretch',
    kind: 'oneshot',
    source: '/vrm/animations/stretch.vrma',
  },
]

export function getBuiltinMotionCatalog(): AvatarMotionEntry[] {
  return BUILTIN_MOTIONS.map((motion) => ({ ...motion }))
}
