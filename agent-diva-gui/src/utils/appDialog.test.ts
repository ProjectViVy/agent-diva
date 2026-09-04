import { describe, expect, it } from 'vitest';
import {
  appConfirmAsync,
  dismissAppDialogConfirm,
  getAppDialogOpen,
  submitAppDialogConfirm,
} from './appDialog';

describe('appConfirmAsync', () => {
  it('keeps the confirmation open while the action is pending', async () => {
    let releaseAction!: () => void;
    const action = new Promise<void>((resolve) => {
      releaseAction = resolve;
    });
    const result = appConfirmAsync('Delete it?', () => action);
    const open = getAppDialogOpen();

    expect(open.value).toMatchObject({ kind: 'confirm', pending: false });

    const submit = submitAppDialogConfirm();
    await Promise.resolve();
    expect(open.value).toMatchObject({ kind: 'confirm', pending: true });

    dismissAppDialogConfirm(false);
    expect(open.value).toMatchObject({ kind: 'confirm', pending: true });

    releaseAction();
    await submit;
    await expect(result).resolves.toBe(true);
    expect(open.value).toBeNull();
  });

  it('keeps the error and allows the action to be retried', async () => {
    let attempts = 0;
    const result = appConfirmAsync('Delete it?', async () => {
      attempts += 1;
      if (attempts === 1) throw new Error('backend unavailable');
    });
    const open = getAppDialogOpen();

    await submitAppDialogConfirm();
    expect(open.value).toMatchObject({
      kind: 'confirm',
      pending: false,
      error: 'backend unavailable',
    });

    await submitAppDialogConfirm();
    await expect(result).resolves.toBe(true);
    expect(open.value).toBeNull();
  });
});
