import { shallowRef, type ShallowRef } from 'vue';

export type AppDialogOpen =
  | {
      kind: 'confirm';
      message: string;
      title?: string;
      confirmLabel?: string;
      cancelLabel?: string;
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
  open.value = null;
  const r = resolveConfirm;
  resolveConfirm = null;
  if (r) r(confirmed);
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
