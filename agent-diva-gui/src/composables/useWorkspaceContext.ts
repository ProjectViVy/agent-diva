import { readonly, shallowRef } from 'vue';
import { getWorkspaceStatus, isTauriRuntime, type WorkspaceStatus } from '../api/desktop';
import { formatDisplayPath } from '../utils/pathDisplay';

export type WorkspaceContextState = 'idle' | 'loading' | 'refreshing' | 'ready' | 'error';

const status = shallowRef<WorkspaceStatus | null>(null);
const state = shallowRef<WorkspaceContextState>('idle');
const error = shallowRef<string | null>(null);
// The gateway reports the resolution source, but a same-root picker choice is
// an explicit session decision even when that root is also the persisted
// default. Keep that intent in the GUI runtime so a later refresh cannot turn
// the active session back into a default-labelled workspace.
const sessionOverrideRoot = shallowRef<string | null>(null);
let refreshGeneration = 0;

function normalizeWorkspaceRoot(root: string): string {
  const normalized = formatDisplayPath(root.trim())
    .replace(/[\\/]+$/, '')
    .replace(/\\/g, '/');
  if (/^[A-Za-z]:\//.test(normalized) || normalized.startsWith('//')) {
    return normalized.toLowerCase();
  }
  return normalized;
}

function sameWorkspaceRoot(left: string, right: string): boolean {
  return normalizeWorkspaceRoot(left) === normalizeWorkspaceRoot(right);
}

function overlaySessionOverride(nextStatus: WorkspaceStatus): WorkspaceStatus {
  const overrideRoot = sessionOverrideRoot.value;
  if (!overrideRoot) return nextStatus;
  if (!sameWorkspaceRoot(overrideRoot, nextStatus.root)) {
    sessionOverrideRoot.value = null;
    return nextStatus;
  }
  return {
    ...nextStatus,
    uses_default_workspace: false,
  };
}

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
    if (nextStatus.source === 'explicit-cli') {
      sessionOverrideRoot.value = nextStatus.root;
    }
    status.value = overlaySessionOverride(nextStatus);
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
  if (nextStatus.source === 'explicit-cli') {
    sessionOverrideRoot.value = nextStatus.root;
  } else if (nextStatus.uses_default_workspace !== false) {
    // A fresh configured/process-cwd status is authoritative at startup. It
    // should not inherit an explicit marker from a previous app runtime.
    sessionOverrideRoot.value = null;
  }
  status.value = overlaySessionOverride(nextStatus);
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
