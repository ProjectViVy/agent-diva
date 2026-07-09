/**
 * useMasks — mask list and lifecycle composable.
 *
 * Provides reactive state for the mask list, the active mask, loading/error
 * indicators, and CRUD operations that call the Tauri backend via desktop.ts
 * API wrappers.
 *
 * State is module-level (singleton), shared across all component instances.
 */

import { ref, type Ref } from 'vue';
import {
  listMasks as apiListMasks,
  getActiveMask as apiGetActiveMask,
  switchMask as apiSwitchMask,
  createOrUpdateMask as apiCreateOrUpdateMask,
  deleteMask as apiDeleteMask,
  isTauriRuntime,
  type MaskEntryDto,
  type MaskPayload,
} from '../api/desktop';

// ---------------------------------------------------------------------------
// Module-level state (shared across component instances)
// ---------------------------------------------------------------------------

const masks: Ref<MaskEntryDto[]> = ref([]);
const activeMask: Ref<MaskEntryDto | null> = ref(null);
const loading = ref(false);
const error: Ref<string | null> = ref(null);

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Composable for mask list and lifecycle management.
 *
 * @example
 * ```ts
 * const { masks, activeMask, refresh, switchTo, create, remove } = useMasks();
 * await refresh();
 * await switchTo('研究员');
 * ```
 */
export function useMasks() {
  /**
   * Refresh the full mask list and the currently active mask from the backend.
   * In mock mode (no Tauri runtime), state remains empty.
   */
  async function refresh(): Promise<void> {
    if (!isTauriRuntime()) {
      return;
    }

    loading.value = true;
    error.value = null;

    try {
      const [maskList, active] = await Promise.all([
        apiListMasks(),
        apiGetActiveMask(),
      ]);
      masks.value = maskList;
      activeMask.value = active;
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  /**
   * Switch the active mask by name.
   * Updates `activeMask` optimistically on success; leaves it unchanged on
   * failure (caller should inspect `error`).
   */
  async function switchTo(name: string): Promise<void> {
    if (!isTauriRuntime()) {
      return;
    }

    loading.value = true;
    error.value = null;

    try {
      const result = await apiSwitchMask(name);
      activeMask.value = result;
      // Refresh the list so the active indicator is up to date
      masks.value = await apiListMasks();
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  /**
   * Create or update a mask with the given payload.
   * Refreshes the full list on success.
   */
  async function create(payload: MaskPayload): Promise<void> {
    if (!isTauriRuntime()) {
      return;
    }

    loading.value = true;
    error.value = null;

    try {
      await apiCreateOrUpdateMask(payload);
      await refresh();
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  /**
   * Delete a mask by name.
   * Refreshes the full list on success. If the deleted mask was the active
   * mask, `activeMask` will be updated by the refresh.
   */
  async function remove(name: string): Promise<void> {
    if (!isTauriRuntime()) {
      return;
    }

    loading.value = true;
    error.value = null;

    try {
      await apiDeleteMask(name);
      await refresh();
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  return {
    masks,
    activeMask,
    loading,
    error,
    refresh,
    switchTo,
    create,
    remove,
  };
}
