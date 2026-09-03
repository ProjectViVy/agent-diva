import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ChannelEditorForm from './ChannelEditorForm.vue';
import { CHANNEL_CREDENTIAL_FIELDS } from './channel-wizard-fields';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({ t: (key: string) => key }),
}));

vi.mock('@lucide/vue', () => {
  const icon = (name: string) => ({ name, template: `<span class="${name}" />` });
  return {
    Eye: icon('Eye'),
    EyeOff: icon('EyeOff'),
  };
});

describe('ChannelEditorForm', () => {
  it('writes booleans as real booleans', async () => {
    const config: Record<string, unknown> = { smtp_use_ssl: false };
    const wrapper = mount(ChannelEditorForm, {
      props: { platform: 'email', config },
      global: { mocks: { $t: (key: string) => key } },
    });

    const details = wrapper.find('details');
    expect(details.exists()).toBe(true);
    await details.find('summary').trigger('click');

    const sslRow = wrapper.findAll('.switch-row').find((row) => row.text().includes('SMTP 使用 SSL'));
    expect(sslRow).toBeTruthy();
    await sslRow!.find('button[role="switch"]').trigger('click');

    expect(config.smtp_use_ssl).toBe(true);
  });

  it('splits string-list text into arrays', async () => {
    const config: Record<string, unknown> = { token: 'x', allow_from: [] };
    const wrapper = mount(ChannelEditorForm, {
      props: { platform: 'telegram', config },
      global: { mocks: { $t: (key: string) => key } },
    });

    await wrapper.find('details summary').trigger('click');
    const textarea = wrapper.find('textarea');
    await textarea.setValue('111\n222, 333');

    expect(config.allow_from).toEqual(['111', '222', '333']);
  });

  it('keeps advanced fields collapsed by default', () => {
    const wrapper = mount(ChannelEditorForm, {
      props: { platform: 'discord', config: { token: 'abc' } },
      global: { mocks: { $t: (key: string) => key } },
    });

    const details = wrapper.find('details');
    expect(details.exists()).toBe(true);
    expect((details.element as HTMLDetailsElement).open).toBe(false);
    expect(wrapper.text()).toContain('channels.advancedSettings');
  });
});

describe('channel credential schema', () => {
  it('covers every production channel with at least one required or defaulted field', () => {
    for (const platform of ['telegram', 'discord', 'feishu', 'dingtalk', 'email', 'qq']) {
      expect(CHANNEL_CREDENTIAL_FIELDS[platform]?.length).toBeGreaterThan(0);
    }
  });
});
