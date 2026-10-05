import { ref, shallowRef, type ShallowRef } from 'vue';

export type AppConfirmOptions = {
  title?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  /** Optional async work that must finish before this confirmation can close. */
  onConfirm?: () => Promise<void>;
};

export type AppDialogOpen =
  | {
      kind: 'confirm';
      message: string;
      title?: string;
      confirmLabel?: string;
      cancelLabel?: string;
      onConfirm?: () => Promise<void>;
    }
  | {
      kind: 'alert';
      message: string;
      title?: string;
      okLabel?: string;
    }
  | {
      kind: 'prompt';
      message: string;
      title?: string;
      confirmLabel?: string;
      cancelLabel?: string;
      placeholder?: string;
    };

const open: ShallowRef<AppDialogOpen | null> = shallowRef(null);
export const appDialogConfirmPending = ref(false);
export const appDialogConfirmError = ref('');

let resolveConfirm: ((v: boolean) => void) | null = null;
let resolveAlert: (() => void) | null = null;
let resolvePrompt: ((v: string | null) => void) | null = null;

function settlePrevious() {
  if (resolveConfirm) {
    const r = resolveConfirm;
    resolveConfirm = null;
    appDialogConfirmPending.value = false;
    appDialogConfirmError.value = '';
    r(false);
  }
  if (resolveAlert) {
    const r = resolveAlert;
    resolveAlert = null;
    r();
  }
  if (resolvePrompt) {
    const r = resolvePrompt;
    resolvePrompt = null;
    r(null);
  }
}

export function getAppDialogOpen(): ShallowRef<AppDialogOpen | null> {
  return open;
}

/** Themed confirm; resolves true if user confirms. */
export function appConfirm(
  message: string,
  options?: AppConfirmOptions,
): Promise<boolean> {
  return new Promise((resolve) => {
    settlePrevious();
    resolveConfirm = resolve;
    appDialogConfirmPending.value = false;
    appDialogConfirmError.value = '';
    open.value = { kind: 'confirm', message, ...options };
  });
}

/** Themed alert; resolves when user acknowledges. */
export function appAlert(
  message: string,
  options?: { title?: string; okLabel?: string },
): Promise<void> {
  return new Promise((resolve) => {
    settlePrevious();
    resolveAlert = resolve;
    open.value = { kind: 'alert', message, ...options };
  });
}

export function dismissAppDialogConfirm(confirmed: boolean) {
  if (appDialogConfirmPending.value && !confirmed) return;
  open.value = null;
  appDialogConfirmPending.value = false;
  appDialogConfirmError.value = '';
  const r = resolveConfirm;
  resolveConfirm = null;
  if (r) r(confirmed);
}

/** Runs an optional async confirm action, leaving its dialog open on failure. */
export async function runAppDialogConfirm() {
  const dialog = open.value;
  if (!dialog || dialog.kind !== 'confirm' || appDialogConfirmPending.value) return;
  if (!dialog.onConfirm) {
    dismissAppDialogConfirm(true);
    return;
  }

  appDialogConfirmPending.value = true;
  appDialogConfirmError.value = '';
  try {
    await dialog.onConfirm();
    if (open.value === dialog) dismissAppDialogConfirm(true);
  } catch (error) {
    if (open.value === dialog) {
      appDialogConfirmError.value = error instanceof Error ? error.message : String(error);
    }
  } finally {
    if (open.value === dialog) appDialogConfirmPending.value = false;
  }
}

export function dismissAppDialogAlert() {
  open.value = null;
  const r = resolveAlert;
  resolveAlert = null;
  if (r) r();
}

/** Themed free-text prompt; resolves with the trimmed input or null if cancelled. */
export function appPrompt(
  message: string,
  options?: { title?: string; confirmLabel?: string; cancelLabel?: string; placeholder?: string },
): Promise<string | null> {
  return new Promise((resolve) => {
    settlePrevious();
    resolvePrompt = resolve;
    open.value = { kind: 'prompt', message, ...options };
  });
}

export function dismissAppDialogPrompt(value: string | null) {
  open.value = null;
  const r = resolvePrompt;
  resolvePrompt = null;
  if (r) r(value);
}
