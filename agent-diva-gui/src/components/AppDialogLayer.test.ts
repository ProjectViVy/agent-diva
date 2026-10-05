import { flushPromises, mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import { describe, expect, it, vi } from 'vitest';
import AppDialogLayer from './AppDialogLayer.vue';
import { appConfirm } from '../utils/appDialog';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}));

describe('AppDialogLayer async confirmations', () => {
  it('keeps the dialog open while work runs, exposes failures, and resolves after retry succeeds', async () => {
    let rejectFirst!: (error: Error) => void;
    const onConfirm = vi.fn()
      .mockImplementationOnce(() => new Promise<void>((_resolve, reject) => { rejectFirst = reject; }))
      .mockResolvedValueOnce(undefined);
    const result = appConfirm('Delete this item?', { onConfirm });
    const wrapper = mount(AppDialogLayer);

    const confirm = document.body.querySelector('.app-dialog-confirm') as HTMLButtonElement;
    confirm.click();
    await nextTick();
    expect(confirm.disabled).toBe(true);
    expect(confirm.textContent).toContain('appDialog.working');
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(document.body.querySelector('[role="dialog"]')).not.toBeNull();
    rejectFirst(new Error('delete failed'));
    await flushPromises();

    expect(document.body.querySelector('[role="alert"]')?.textContent).toContain('delete failed');
    expect(document.body.querySelector('.app-dialog-confirm')).not.toBeNull();
    expect(onConfirm).toHaveBeenCalledTimes(1);

    (document.body.querySelector('.app-dialog-confirm') as HTMLButtonElement).click();
    await flushPromises();

    await expect(result).resolves.toBe(true);
    expect(onConfirm).toHaveBeenCalledTimes(2);
    expect(document.body.querySelector('[role="dialog"]')).toBeNull();
    wrapper.unmount();
  });
});
