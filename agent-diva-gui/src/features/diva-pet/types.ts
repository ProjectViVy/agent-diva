/** Message role for Diva Pet display */
export interface PetMessage {
  role: 'user' | 'agent' | 'system' | 'tool'
  content: string
  timestamp?: number
  emotion?: string
  isStreaming?: boolean
  isThinking?: boolean
}

/** VRM expression / mood identifier */
export type VrmMood = 'neutral' | 'happy' | 'sad' | 'angry' | 'surprised'

/** VRM model info (matches Tauri command output) */
export interface VrmModelInfo {
  /** Unique identifier (filename without extension) */
  id: string
  /** Display name */
  name: string
  /** Builtin URL or custom relative path under ~/.agent-diva/vrm/models/ */
  path: string
  /** Model source */
  source: 'builtin' | 'custom'
  /** Optional thumbnail path */
  thumbnail?: string
}

/** VRM loading state */
export type VrmLoadState = 'idle' | 'loading' | 'loaded' | 'error'

// ── Phase 1 新增类型: VRM 动画/外观配置 ──────────────────────────

/**
 * VRMA motion file information.
 * Adapted from super-agent-party's VRMConfig.vrm_motion_list.
 */
export interface VrmMotionInfo {
  /** Unique identifier (filename without extension) */
  id: string
  /** Display name (used for AI semantic motion calling) */
  name: string
  /** Runtime motion kind */
  kind?: 'idle' | 'oneshot'
  /** Relative path (e.g. /vrm/animations/greeting.vrma) */
  path: string
  /** Optional thumbnail path */
  thumbnail?: string
}

/**
 * Multi-appearance configuration.
 * Adapted from super-agent-party's VRMConfig.name-preset system.
 */
export interface VrmAppearanceConfig {
  /** Unique appearance identifier */
  id: string
  /** Human-readable name */
  name: string
  /** Bound VRM model ID (maps to VrmModelInfo.id) */
  modelId: string
  /** Selected VRMA motion IDs for this appearance */
  motionIds: string[]
  /** Whether expression override is enabled */
  expressionEnabled: boolean
  /** Whether idle motion is enabled */
  motionEnabled: boolean
  /** Optional window width for this appearance */
  windowWidth?: number
  /** Optional window height for this appearance */
  windowHeight?: number
}

/** 3D world-space target for look-at tracking */
export interface VrmLookAtTarget {
  x: number
  y: number
  z: number
}

// ── 3D Gaussian Splatting 场景配置 ────────────────────────────

/**
 * Predefined scene identifiers for 3D Gaussian Splatting background.
 * 'transparent' means no background (transparent/alpha).
 */
export type GaussSceneId = 'transparent' | 'space' | 'home' | 'sea'

/**
 * A 3D Gaussian Splatting scene entry.
 * Matches the scene format used in avatar-runtime-vrm.
 */
export interface GaussSceneEntry {
  /** Scene identifier */
  id: GaussSceneId | string
  /** Human-readable display name */
  name: string
  /** Relative path under public/ (empty string for transparent) */
  path: string
  /** Whether this is a built-in default scene */
  isDefault: boolean
}

// ── PetConfig 扩展 ─────────────────────────────────────────────

/** Pet configuration shape */
export interface PetConfig {
  /** Master switch for Diva Pet sidebar entry */
  enabled: boolean
  /** Selected VRM model filename (relative to public/vrm/models/) */
  vrmModel: string
  /** Whether TTS auto-play is enabled */
  ttsEnabled: boolean
  /** Whether ASR / microphone input is enabled */
  asrEnabled: boolean
  /** ASR provider */
  asrProvider: 'web_speech' | 'siliconflow'
  /** ASR language tag */
  asrLanguage: string
  /** API key for remote ASR providers */
  asrApiKey: string | null
  /** Base URL for remote ASR providers */
  asrBaseUrl: string
  /** Model for remote ASR providers */
  asrModel: string | null
  /** TTS provider */
  ttsProvider: 'browser' | 'openai' | 'siliconflow' | 'minimax'
  /** Legacy shared TTS API key. Kept only for old local data and ignored by new logic. */
  ttsApiKey: string | null
  /** Provider-specific API keys used by the current GUI logic. */
  ttsOpenaiApiKey: string | null
  ttsSiliconflowApiKey: string | null
  ttsMinimaxApiKey: string | null
  /** Base URL for remote TTS providers */
  ttsBaseUrl: string
  /** Model for remote TTS providers */
  ttsModel: string | null
  /** Provider-specific system voice identifier */
  ttsVoiceId: string | null
  /** Relative voice reference path under voice_resource/ */
  ttsReferenceVoice: string | null
  /** Transcript for the reference voice clip */
  ttsReferenceText: string | null
  /** TTS playback speed */
  ttsSpeed: number
  /** TTS playback volume */
  ttsVolume: number

  // ===== Phase 1 新增: VRM 动画配置 =====
  /** Whether VRMA idle animation is enabled */
  vrmMotionEnabled: boolean
  /** Available VRMA motion list (scanned from /public/vrm/animations/) */
  vrmMotionList: VrmMotionInfo[]
  /** Currently selected motion IDs for idle loop */
  selectedMotionIds: string[]

  // ===== Phase 1 新增: VRM 表情配置 =====
  /** Whether custom expression mapping is enabled */
  vrmExpressionEnabled: boolean
  /** Expression intensity (0-1) */
  vrmExpressionIntensity: number

  // ===== Phase 1 新增: VRM 外观配置 =====
  /** Saved appearance configurations */
  vrmAppearances: VrmAppearanceConfig[]
  /** Currently active appearance ID */
  activeAppearanceId: string

  // ===== Phase 1 新增: VRM 交互配置 =====
  /** Whether auto-hide on hover is enabled (desktop-pet mode) */
  vrmAutoHideEnabled: boolean
  /** Whether look-at tracking is enabled */
  vrmLookAtEnabled: boolean

  // ===== Desktop Pet 菜单增强 =====
  /** Desktop pet scale factor (0.75 - 1.6) */
  desktopPetScale: number
  /** Whether desktop pet window is always-on-top */
  desktopPetAlwaysOnTop: boolean
  /** Whether subtitle overlay is enabled */
  subtitleEnabled: boolean

  // ===== 3D Gaussian Splatting 背景场景 =====
  /** Selected 3D Gauss scene ID */
  selectedGaussSceneId: GaussSceneId
  /** Available 3D Gauss scene presets */
  gaussSceneList: GaussSceneEntry[]
}

/** Default pet configuration */
export const DEFAULT_PET_CONFIG: PetConfig = {
  enabled: true,
  vrmModel: '',
  ttsEnabled: false,
  asrEnabled: true,
  asrProvider: 'web_speech',
  asrLanguage: 'zh-CN',
  asrApiKey: null,
  asrBaseUrl: '',
  asrModel: null,
  ttsProvider: 'browser',
  ttsApiKey: null,
  ttsOpenaiApiKey: null,
  ttsSiliconflowApiKey: null,
  ttsMinimaxApiKey: null,
  ttsBaseUrl: '',
  ttsModel: null,
  ttsVoiceId: null,
  ttsReferenceVoice: null,
  ttsReferenceText: null,
  ttsSpeed: 1.0,
  ttsVolume: 1.0,

  // Phase 1 defaults — all new features off by default for backward compatibility
  vrmMotionEnabled: false,
  vrmMotionList: [],
  selectedMotionIds: [],

  vrmExpressionEnabled: false,
  vrmExpressionIntensity: 0.85,

  vrmAppearances: [],
  activeAppearanceId: 'default',

  vrmAutoHideEnabled: false,
  vrmLookAtEnabled: false,

  // Desktop Pet 菜单增强
  desktopPetScale: 1.0,
  desktopPetAlwaysOnTop: true,
  subtitleEnabled: true,

  // 3D Gaussian Splatting 背景场景 — 默认透明背景
  selectedGaussSceneId: 'transparent' as GaussSceneId,
  gaussSceneList: [
    { id: 'transparent', name: '透明背景', path: '', isDefault: true },
    { id: 'home',        name: '室内场景', path: 'vrm/scene/home.spz',  isDefault: true },
    { id: 'sea',         name: '海边场景', path: 'vrm/scene/sea.spz',   isDefault: true },
    { id: 'space',       name: '太空场景', path: 'vrm/scene/space.spz', isDefault: true },
  ],
}

/**
 * Resolve the effective TTS API key for the currently selected provider.
 * New GUI logic only accepts provider-specific keys.
 */
export function getTtsApiKey(config: PetConfig): string | null {
  switch (config.ttsProvider) {
    case 'minimax':
      return config.ttsMinimaxApiKey
    case 'siliconflow':
      return config.ttsSiliconflowApiKey
    case 'openai':
      return config.ttsOpenaiApiKey
    default:
      return null
  }
}
