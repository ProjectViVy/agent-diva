import { shallowRef, type ShallowRef } from 'vue';

export type AppDialogOpen =
  | {
      kind: 'confirm';
      message: string;
      title?: string;
      confirmLabel?: string;
      cancelLabel?: string;
      pending?: boolean;
      error?: string | null;
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

let resolveConfirm: ((v: boolean) => void) | null = null;
let resolveAlert: (() => void) | null = null;
let resolvePrompt: ((v: string | null) => void) | null = null;
let confirmAction: (() => Promise<void>) | null = null;
let confirmPending = false;
let confirmError: string | null = null;

function publishConfirmState() {
  if (open.value?.kind !== 'confirm') return;
  open.value = {
    ...open.value,
    pending: confirmPending,
    error: confirmError,
  };
}

function resetConfirmState() {
  confirmAction = null;
  confirmPending = false;
  confirmError = null;
}

function settlePrevious() {
  if (resolveConfirm) {
    const r = resolveConfirm;
    resolveConfirm = null;
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
  resetConfirmState();
}

export function getAppDialogOpen(): ShallowRef<AppDialogOpen | null> {
  return open;
}

/** Themed confirm; resolves true if user confirms. */
export function appConfirm(
  message: string,
  options?: { title?: string; confirmLabel?: string; cancelLabel?: string },
): Promise<boolean> {
  return new Promise((resolve) => {
    settlePrevious();
    resolveConfirm = resolve;
    resetConfirmState();
    open.value = { kind: 'confirm', message, ...options };
  });
}

/**
 * Themed confirmation for an asynchronous action.
 *
 * The dialog stays open while `action` is running. A rejected action leaves
 * the dialog open with its error and can be retried by confirming again.
 */
export function appConfirmAsync(
  message: string,
  action: () => Promise<void>,
  options?: { title?: string; confirmLabel?: string; cancelLabel?: string },
): Promise<boolean> {
  return new Promise((resolve) => {
    settlePrevious();
    resolveConfirm = resolve;
    confirmAction = action;
    confirmPending = false;
    confirmError = null;
    open.value = {
      kind: 'confirm',
      message,
      ...options,
      pending: false,
      error: null,
    };
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
  if (confirmPending) return;
  if (confirmed && confirmAction) {
    void submitAppDialogConfirm();
    return;
  }
  open.value = null;
  const r = resolveConfirm;
  resolveConfirm = null;
  resetConfirmState();
  if (r) r(confirmed);
}

/** Start or retry the action attached to the active async confirmation. */
export async function submitAppDialogConfirm(): Promise<void> {
  if (open.value?.kind !== 'confirm' || confirmPending) return;
  if (!confirmAction) {
    dismissAppDialogConfirm(true);
    return;
  }

  confirmPending = true;
  confirmError = null;
  publishConfirmState();

  try {
    await confirmAction();
    confirmAction = null;
    confirmPending = false;
    confirmError = null;
    dismissAppDialogConfirm(true);
  } catch (error) {
    confirmPending = false;
    confirmError = error instanceof Error ? error.message : String(error);
    publishConfirmState();
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
