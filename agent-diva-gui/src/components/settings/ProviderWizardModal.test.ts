import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import ProviderWizardModal from './ProviderWizardModal.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, values?: Record<string, string>) =>
      values?.state ? `${key} ${values.state}` : key,
  }),
}))

vi.mock('@lucide/vue', () => {
  const icon = (name: string) => ({ name, template: `<span class="${name}" />` })
  return {
    X: icon('X'),
    ChevronRight: icon('ChevronRight'),
    LoaderCircle: icon('LoaderCircle'),
    Check: icon('Check'),
    CircleAlert: icon('CircleAlert'),
    PlugZap: icon('PlugZap'),
    Eye: icon('Eye'),
    EyeOff: icon('EyeOff'),
  }
})

describe('ProviderWizardModal capability gate', () => {
  it('cannot test or complete a deferred provider selection', async () => {
    const wrapper = mount(ProviderWizardModal, {
      props: {
        open: true,
        providers: [
          {
            name: 'deferred',
            display_name: 'Deferred Responses',
            default_api_base: 'https://responses.example/v1',
            executable: false,
            capability_state: 'DEFERRED-INDEFINITE',
          },
          {
            name: 'supported',
            display_name: 'Supported Provider',
            default_api_base: 'https://supported.example/v1',
            executable: true,
            capability_state: 'UNCONFIGURED',
          },
        ],
        initialData: {
          selectedProvider: 'deferred',
          apiKey: 'typed-secret',
          apiBase: 'https://responses.example/v1',
        },
      },
      attachTo: document.body,
    })

    await flushPromises()
    const deferredOption = document.body.querySelector('option[value="deferred"]')
    expect(deferredOption?.hasAttribute('disabled')).toBe(true)
    expect(document.body.textContent).toContain('providers.capabilityUnavailable DEFERRED-INDEFINITE')

    const next = document.body.querySelector<HTMLButtonElement>('.wizard-btn-primary')
    expect(next?.disabled).toBe(true)
    next?.click()
    await flushPromises()

    expect(document.body.querySelector('.wizard-test-btn')).toBeNull()
    expect(wrapper.emitted('test')).toBeUndefined()
    expect(wrapper.emitted('complete')).toBeUndefined()
    wrapper.unmount()
  })
})
