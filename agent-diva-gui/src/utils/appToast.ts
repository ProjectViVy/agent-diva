import { shallowRef, type ShallowRef } from 'vue';

export type AppToastTone = 'success' | 'error';

export interface AppToastState {
  id: number;
  tone: AppToastTone;
  message: string;
}

const activeToast: ShallowRef<AppToastState | null> = shallowRef(null);

let nextToastId = 1;
let dismissTimer: ReturnType<typeof setTimeout> | null = null;

function clearDismissTimer() {
  if (dismissTimer) {
    clearTimeout(dismissTimer);
    dismissTimer = null;
  }
}

export function getAppToast(): ShallowRef<AppToastState | null> {
  return activeToast;
}

export function showAppToast(
  message: string,
  tone: AppToastTone = 'success',
  durationMs = 2400,
) {
  clearDismissTimer();
  const nextToast: AppToastState = {
    id: nextToastId++,
    tone,
    message,
  };
  activeToast.value = nextToast;
  dismissTimer = setTimeout(() => {
    if (activeToast.value?.id === nextToast.id) {
      activeToast.value = null;
    }
    dismissTimer = null;
  }, durationMs);
}

export function dismissAppToast() {
  clearDismissTimer();
  activeToast.value = null;
}
