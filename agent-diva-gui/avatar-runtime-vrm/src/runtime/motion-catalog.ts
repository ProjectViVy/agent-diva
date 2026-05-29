import type { AvatarMotionEntry } from '../protocol'

const BUILTIN_MOTIONS: AvatarMotionEntry[] = [
  {
    id: 'akimbo',
    name: 'Akimbo',
    kind: 'idle',
    source: '/vrm/animations/akimbo.vrma',
  },
  {
    id: 'LookAround',
    name: 'Look Around',
    kind: 'idle',
    source: '/vrm/animations/LookAround.vrma',
  },
  {
    id: 'model_pose',
    name: 'Model Pose',
    kind: 'idle',
    source: '/vrm/animations/model_pose.vrma',
  },
  {
    id: 'Relax',
    name: 'Relax',
    kind: 'idle',
    source: '/vrm/animations/Relax.vrma',
  },
  {
    id: 'show_full_body',
    name: 'Show Full Body',
    kind: 'idle',
    source: '/vrm/animations/show_full_body.vrma',
  },
  {
    id: 'Sleepy',
    name: 'Sleepy',
    kind: 'idle',
    source: '/vrm/animations/Sleepy.vrma',
  },
  {
    id: 'waiting',
    name: 'Waiting',
    kind: 'idle',
    source: '/vrm/animations/waiting.vrma',
  },
  {
    id: 'Angry',
    name: 'Angry',
    kind: 'oneshot',
    source: '/vrm/animations/Angry.vrma',
  },
  {
    id: 'appearing',
    name: 'Appearing',
    kind: 'oneshot',
    source: '/vrm/animations/appearing.vrma',
  },
  {
    id: 'Blush',
    name: 'Blush',
    kind: 'oneshot',
    source: '/vrm/animations/Blush.vrma',
  },
  {
    id: 'Clapping',
    name: 'Clapping',
    kind: 'oneshot',
    source: '/vrm/animations/Clapping.vrma',
  },
  {
    id: 'Goodbye',
    name: 'Goodbye',
    kind: 'oneshot',
    source: '/vrm/animations/Goodbye.vrma',
  },
  {
    id: 'greeting',
    name: 'Greeting',
    kind: 'oneshot',
    source: '/vrm/animations/greeting.vrma',
  },
  {
    id: 'Jump',
    name: 'Jump',
    kind: 'oneshot',
    source: '/vrm/animations/Jump.vrma',
  },
  {
    id: 'liked',
    name: 'Liked',
    kind: 'oneshot',
    source: '/vrm/animations/liked.vrma',
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
    id: 'Sad',
    name: 'Sad',
    kind: 'oneshot',
    source: '/vrm/animations/Sad.vrma',
  },
  {
    id: 'scratch_head',
    name: 'Scratch Head',
    kind: 'oneshot',
    source: '/vrm/animations/scratch_head.vrma',
  },
  {
    id: 'shoot',
    name: 'Shoot',
    kind: 'oneshot',
    source: '/vrm/animations/shoot.vrma',
  },
  {
    id: 'spin',
    name: 'Spin',
    kind: 'oneshot',
    source: '/vrm/animations/spin.vrma',
  },
  {
    id: 'squat',
    name: 'Squat',
    kind: 'oneshot',
    source: '/vrm/animations/squat.vrma',
  },
  {
    id: 'stretch',
    name: 'Stretch',
    kind: 'oneshot',
    source: '/vrm/animations/stretch.vrma',
  },
  {
    id: 'Surprised',
    name: 'Surprised',
    kind: 'oneshot',
    source: '/vrm/animations/Surprised.vrma',
  },
  {
    id: 'Thinking',
    name: 'Thinking',
    kind: 'oneshot',
    source: '/vrm/animations/Thinking.vrma',
  },
]

export function getBuiltinMotionCatalog(): AvatarMotionEntry[] {
  return BUILTIN_MOTIONS.map((motion) => ({ ...motion }))
}
