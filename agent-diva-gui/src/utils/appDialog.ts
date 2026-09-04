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
let confirmOwner: number | null = null;
let dialogGeneration = 0;
let confirmPending = false;
let confirmError: string | null = null;

function publishConfirmState(owner?: number) {
  if (open.value?.kind !== 'confirm' || (owner !== undefined && confirmOwner !== owner)) return;
  open.value = {
    ...open.value,
    pending: confirmPending,
    error: confirmError,
  };
}

function resetConfirmState() {
  confirmAction = null;
  confirmOwner = null;
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

function beginDialog(): number {
  settlePrevious();
  dialogGeneration += 1;
  return dialogGeneration;
}

function isActiveConfirm(owner?: number | null): boolean {
  if (open.value?.kind !== 'confirm') return false;
  return owner === undefined || confirmOwner === owner;
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
    beginDialog();
    resolveConfirm = resolve;
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
    const owner = beginDialog();
    resolveConfirm = resolve;
    confirmOwner = owner;
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
    beginDialog();
    resolveAlert = resolve;
    open.value = { kind: 'alert', message, ...options };
  });
}

export function dismissAppDialogConfirm(confirmed: boolean) {
  if (!isActiveConfirm()) return;
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
  const owner = confirmOwner;
  if (!isActiveConfirm(owner) || confirmPending) return;
  if (!confirmAction) {
    dismissAppDialogConfirm(true);
    return;
  }
  if (owner === null) return;

  const action = confirmAction;

  confirmPending = true;
  confirmError = null;
  publishConfirmState(owner);

  try {
    await action();
    if (!isActiveConfirm(owner)) return;
    confirmAction = null;
    confirmPending = false;
    confirmError = null;
    dismissAppDialogConfirm(true);
  } catch (error) {
    if (!isActiveConfirm(owner)) return;
    confirmPending = false;
    confirmError = error instanceof Error ? error.message : String(error);
    publishConfirmState(owner);
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
    beginDialog();
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
