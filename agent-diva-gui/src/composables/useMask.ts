/**
 * useMask — mask state management composable.
 *
 * Manages the currently active mask and the list of available masks.
 * In mock mode (Tauri not available or backend commands not yet wired),
 * returns hardcoded sample masks. Once Tauri IPC commands for masks are
 * implemented, swap the mock calls for real `invoke()` calls.
 */

import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { isTauriRuntime } from '../api/desktop';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Minimal mask representation for the GUI. */
export interface MaskEntry {
  name: string;
  icon: string;
  description: string;
}

// ---------------------------------------------------------------------------
// Module state (shared across component instances)
// ---------------------------------------------------------------------------

/** The currently active mask. */
const currentMask = ref<MaskEntry>({
  name: '我就是我',
  icon: '😊',
  description: '默认身份，无特殊设定',
});

/** All available masks (including default). */
const masks = ref<MaskEntry[]>([]);

/** Whether the initial load has completed. */
const loaded = ref(false);

// ---------------------------------------------------------------------------
// Mock data (used when Tauri is unavailable)
// ---------------------------------------------------------------------------

const MOCK_MASKS: MaskEntry[] = [
  { name: '我就是我', icon: '😊', description: '默认身份，无特殊设定' },
  { name: '研究员', icon: '🔍', description: '专注调研与分析' },
  { name: 'Rust Coder', icon: '🦀', description: 'Rust 编程专家' },
  { name: '助手', icon: '🤖', description: '通用助手' },
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Composable for mask state management.
 *
 * @example
 * ```ts
 * const { currentMask, masks, switchMask, loadMasks } = useMask();
 * await loadMasks();
 * await switchMask('研究员');
 * ```
 */
export function useMask() {
  /**
   * Load all available masks from the backend.
   * Falls back to mock data when Tauri is not available.
   */
  async function loadMasks(): Promise<void> {
    if (!isTauriRuntime()) {
      masks.value = MOCK_MASKS;
      loaded.value = true;
      return;
    }

    try {
      const result = await invoke<MaskEntry[]>('list_masks');
      masks.value = result;
    } catch {
      // Backend command not yet wired — use mock data
      masks.value = MOCK_MASKS;
    }

    // Load current mask
    try {
      const current = await invoke<MaskEntry | null>('get_current_mask');
      if (current) {
        currentMask.value = current;
      }
    } catch {
      // Keep default
    }

    loaded.value = true;
  }

  /**
   * Switch to a mask by name.
   * Updates local state optimistically; reverts on failure.
   */
  async function switchMask(name: string): Promise<void> {
    const target = masks.value.find((m) => m.name === name);
    if (!target) return;

    const previous = currentMask.value;
    currentMask.value = target;

    if (!isTauriRuntime()) {
      // Mock mode — just update local state
      return;
    }

    try {
      await invoke('switch_mask', { name });
    } catch {
      // Revert on failure
      currentMask.value = previous;
    }
  }

  /** Emoji icon of the current mask (convenience computed). */
  const currentIcon = computed(() => currentMask.value.icon);

  /** Display name of the current mask. */
  const currentName = computed(() => currentMask.value.name);

  return {
    currentMask,
    currentIcon,
    currentName,
    masks,
    loaded,
    loadMasks,
    switchMask,
  };
}
