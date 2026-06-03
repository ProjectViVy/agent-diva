/**
 * useLipSync — Vue 3 composable for Phase 3 lip sync management.
 *
 * Wraps {@link LipSyncAnalyzer} and coordinates:
 *   - Audio source connection (HTMLAudioElement or raw AudioNode)
 *   - Reactive viseme driving via avatar-runtime-vrm's setSpeechState()
 *   - Lifecycle management tied to speaking state
 *
 * Reference: super-agent-party vrm.js:1537-1704
 *
 * Integration: Uses AvatarSpeechState.viseme to drive mouth
 *              BlendShapes through the VRM runtime's SpeechController.
 */

import { ref, shallowRef, watch, type Ref, type ShallowRef } from 'vue'
import type { AvatarSpeechState } from 'avatar-runtime-vrm'
import {
  LipSyncAnalyzer,
  type VowelBlends,
  type LipSyncConfig,
} from './lip-sync-analyzer'

// ── Runtime interface (minimal) ──────────────────────────────────

/** Minimal VRM runtime interface for speech state control */
export interface LipSyncRuntimeHandle {
  setSpeechState(state: Partial<AvatarSpeechState>): Promise<void> | void
}

// ── Composable options ────────────────────────────────────────────

export interface UseLipSyncOptions {
  /** VRM runtime reference (shallow ref) */
  runtime: ShallowRef<LipSyncRuntimeHandle | null>
  /** Whether lip sync is enabled globally */
  enabled?: Ref<boolean> | boolean
  /** Lip sync analyzer configuration */
  config?: Partial<LipSyncConfig>
  /** Whether to auto-connect when a new HTMLAudioElement is detected */
  autoConnect?: boolean
}

// ── Composable result ────────────────────────────────────────────

export interface UseLipSyncResult {
  /** The underlying LipSyncAnalyzer instance (shallow ref) */
  analyzer: ShallowRef<LipSyncAnalyzer | null>
  /** Current vowel blends (reactive) */
  blends: Ref<VowelBlends>
  /** Dominant viseme string (aa/ih/ou/ee/oh) or null if silent */
  viseme: Ref<string | null>
  /** Whether the analysis loop is currently running */
  isAnalyzing: Ref<boolean>
  /** Connect an HTMLAudioElement as the audio source */
  connectAudioElement: (audioElement: HTMLAudioElement) => void
  /** Connect a raw AudioNode as the audio source (for mic input) */
  connectAudioNode: (sourceNode: AudioNode) => void
  /** Start the analysis loop */
  start: () => void
  /** Stop the analysis loop */
  stop: () => void
  /** Clean up all resources */
  destroy: () => void
}

// ── Composable implementation ────────────────────────────────────

export function useLipSync(options: UseLipSyncOptions): UseLipSyncResult {
  const { runtime, enabled, config, autoConnect = false } = options

  // ── State ─────────────────────────────────────────────────────

  const analyzer = shallowRef<LipSyncAnalyzer | null>(null)
  const blends = ref<VowelBlends>({ aa: 0, ih: 0, ou: 0, ee: 0, oh: 0 })
  const viseme = ref<string | null>(null)
  const isAnalyzing = ref(false)
  const isEnabled = typeof enabled === 'boolean' ? enabled : (enabled ?? ref(true))

  // ── Init ──────────────────────────────────────────────────────

  analyzer.value = new LipSyncAnalyzer(config)

  // Throttle viseme updates to avoid overwhelming the runtime
  let lastViseme: string | null = null
  let visemeDebounceTimer: ReturnType<typeof setTimeout> | null = null

  function updateViseme(newViseme: string | null): void {
    // Only send if viseme actually changed (prevents redundant calls)
    if (newViseme === lastViseme) return
    lastViseme = newViseme

    const rt = runtime.value
    if (!rt) return

    const speechState: Partial<AvatarSpeechState> = {
      speaking: true,
      viseme: newViseme,
      intensity: newViseme ? 0.8 : 0.3,
    }

    try {
      void rt.setSpeechState(speechState)
    } catch (error) {
      console.warn('[useLipSync] Failed to update speech state', error)
    }
  }

  // ── Blend update handler ──────────────────────────────────────

  function onBlendUpdate(newBlends: VowelBlends): void {
    blends.value = { ...newBlends }
    const dominant = LipSyncAnalyzer.getDominantViseme(newBlends, 0.05)
    viseme.value = dominant
    updateViseme(dominant)
  }

  // ── Public API ────────────────────────────────────────────────

  function connectAudioElement(audioElement: HTMLAudioElement): void {
    analyzer.value?.connectMediaElement(audioElement)
  }

  function connectAudioNode(sourceNode: AudioNode): void {
    analyzer.value?.connectNode(sourceNode)
  }

  function start(): void {
    const enabledVal = typeof isEnabled === 'object' ? isEnabled.value : isEnabled
    if (!enabledVal) {
      console.log('[useLipSync] Lip sync disabled, skipping start')
      return
    }

    analyzer.value?.start(onBlendUpdate)
    isAnalyzing.value = true
  }

  function stop(): void {
    analyzer.value?.stop()
    isAnalyzing.value = false
    // Reset speech state to non-viseme mode
    updateViseme(null)
    lastViseme = null
  }

  function destroy(): void {
    stop()
    if (visemeDebounceTimer !== null) {
      clearTimeout(visemeDebounceTimer)
      visemeDebounceTimer = null
    }
    // Run all registered cleanup hooks
    for (const hook of cleanupHooks) {
      hook()
    }
    analyzer.value?.destroy()
    analyzer.value = null
  }

  // ── Auto-connect watcher (for TTS audio) ──────────────────────

  // Cleanup hooks for resources added by feature watchers
  const cleanupHooks: Array<() => void> = []

  if (autoConnect) {
    // Periodically check for new audio elements in the DOM
    // (fires when the TTS service creates a new HTMLAudioElement)
    let pollInterval: ReturnType<typeof setInterval> | null = null

    watch(
      () => (typeof isEnabled === 'object' ? isEnabled.value : isEnabled),
      (enabledVal) => {
        if (enabledVal) {
          pollInterval = setInterval(() => {
            const audioElements = document.querySelectorAll('audio')
            for (const el of audioElements) {
              if (!el.dataset.lipSyncConnected && !el.paused) {
                el.dataset.lipSyncConnected = 'true'
                connectAudioElement(el)
                break
              }
            }
          }, 200) // Check every 200ms
        } else {
          if (pollInterval !== null) {
            clearInterval(pollInterval)
            pollInterval = null
          }
        }
      },
      { immediate: true },
    )

    // Register interval cleanup for destroy
    cleanupHooks.push(() => {
      if (pollInterval !== null) {
        clearInterval(pollInterval)
        pollInterval = null
      }
    })
  }

  // ── Expose ────────────────────────────────────────────────────

  return {
    analyzer,
    blends,
    viseme,
    isAnalyzing,
    connectAudioElement,
    connectAudioNode,
    start,
    stop,
    destroy,
  }
}
