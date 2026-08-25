import { readonly, shallowRef } from 'vue';
import { getWorkspaceStatus, isTauriRuntime, type WorkspaceStatus } from '../api/desktop';

export type WorkspaceContextState = 'idle' | 'loading' | 'refreshing' | 'ready' | 'error';

const status = shallowRef<WorkspaceStatus | null>(null);
const state = shallowRef<WorkspaceContextState>('idle');
const error = shallowRef<string | null>(null);
let refreshGeneration = 0;

async function refresh(): Promise<boolean> {
  if (!isTauriRuntime()) {
    state.value = 'ready';
    return false;
  }

  const generation = ++refreshGeneration;
  state.value = status.value ? 'refreshing' : 'loading';
  error.value = null;

  try {
    const nextStatus = await getWorkspaceStatus();
    if (generation !== refreshGeneration) return false;
    status.value = nextStatus;
    state.value = 'ready';
    return true;
  } catch (cause) {
    if (generation !== refreshGeneration) return false;
    state.value = 'error';
    error.value = cause instanceof Error ? cause.message : String(cause);
    return false;
  }
}

function apply(nextStatus: WorkspaceStatus) {
  refreshGeneration += 1;
  status.value = nextStatus;
  state.value = 'ready';
  error.value = null;
}

export function useWorkspaceContext() {
  return {
    status: readonly(status),
    state: readonly(state),
    error: readonly(error),
    refresh,
    apply,
  };
}
