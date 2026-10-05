import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SpeechSettings from './SpeechSettings.vue';
import { speechConfig, voiceAssets } from '../../api/speech';
import { appConfirm } from '../../utils/appDialog';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}));

vi.mock('@lucide/vue', () => {
  const icon = (name: string) => ({ name, template: `<span class="${name}" />` });
  return { Mic: icon('Mic'), LoaderCircle: icon('LoaderCircle'), Trash2: icon('Trash2') };
});

vi.mock('../../api/speech', () => ({
  NativeUnavailableError: class NativeUnavailableError extends Error {},
  speechConfig: { get: vi.fn(), update: vi.fn() },
  speechCredentials: { set: vi.fn(), delete: vi.fn() },
  voiceAssets: { list: vi.fn(), import: vi.fn(), delete: vi.fn() },
}));

vi.mock('../../utils/appDialog', () => ({ appConfirm: vi.fn() }));
vi.mock('../../utils/appToast', () => ({ showAppToast: vi.fn() }));
vi.mock('../../state/gui-diagnostics', () => ({ recordGuiDiagnostic: vi.fn() }));

const preferences = {
  stt: { provider: 'siliconflow', base_url: 'https://stt.example', model: 'stt-model' },
  tts: {
    provider: 'siliconflow',
    siliconflow: { base_url: 'https://tts.example', model: 'tts-model', voice: 'voice-1', speed: 1 },
    minimax: { base_url: '', model: '', voice_id: '', speed: 1, volume: 1 },
  },
  auto_read_replies: false,
};

const readback = (revision: number, value = preferences) => ({
  schema: 'speech.v1',
  revision,
  preferences: value,
  credential_state: { siliconflow: 'missing', minimax: 'missing' },
  window_context: null,
});

describe('SpeechSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(speechConfig.get).mockResolvedValue(readback(1));
    vi.mocked(voiceAssets.list).mockResolvedValue([]);
  });

  it('preserves a failed-save draft and only replaces it after explicit reload confirmation', async () => {
    vi.mocked(speechConfig.update).mockRejectedValueOnce(new Error('revision conflict'));
    vi.mocked(appConfirm).mockResolvedValue(false);

    const wrapper = mount(SpeechSettings);
    await flushPromises();
    const baseUrl = wrapper.find('input[placeholder="speechSettings.baseUrlPlaceholder"]');
    await baseUrl.setValue('https://draft.example');
    await wrapper.find('button.ui-button--primary').trigger('click');
    await flushPromises();

    expect(wrapper.find('[role="alert"]').text()).toContain('revision conflict');
    expect((baseUrl.element as HTMLInputElement).value).toBe('https://draft.example');
    const reload = wrapper.findAll('button').find((button) => button.text() === 'speechSettings.reloadAction');
    expect(reload).toBeDefined();

    await reload!.trigger('click');
    await flushPromises();
    expect(appConfirm).toHaveBeenCalledWith('speechSettings.reloadConfirm', {
      title: 'speechSettings.reloadTitle',
      confirmLabel: 'speechSettings.reloadAction',
      cancelLabel: 'appDialog.cancel',
    });
    expect(speechConfig.get).toHaveBeenCalledTimes(1);
    expect((baseUrl.element as HTMLInputElement).value).toBe('https://draft.example');

    vi.mocked(appConfirm).mockResolvedValueOnce(true);
    vi.mocked(speechConfig.get).mockResolvedValueOnce(readback(2));
    await reload!.trigger('click');
    await flushPromises();
    expect(speechConfig.get).toHaveBeenCalledTimes(2);
    expect(wrapper.find('[role="alert"]').exists()).toBe(false);
    expect(wrapper.find('input[placeholder="speechSettings.baseUrlPlaceholder"]').element).toHaveProperty('value', 'https://stt.example');
    wrapper.unmount();
  });
});
