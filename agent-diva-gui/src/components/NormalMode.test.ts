import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';
import NormalMode from './NormalMode.vue';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('lucide-vue-next', () => ({
  AlarmClock: { name: 'AlarmClock', template: '<span class="AlarmClock" />' },
  BookOpen: { name: 'BookOpen', template: '<span class="BookOpen" />' },
  Bot: { name: 'Bot', template: '<span class="Bot" />' },
  Cat: { name: 'Cat', template: '<span class="Cat" />' },
  Check: { name: 'Check', template: '<span class="Check" />' },
  ChevronDown: { name: 'ChevronDown', template: '<span class="ChevronDown" />' },
  GitBranch: { name: 'GitBranch', template: '<span class="GitBranch" />' },
  Heart: { name: 'Heart', template: '<span class="Heart" />' },
  Menu: { name: 'Menu', template: '<span class="Menu" />' },
  MessageSquare: { name: 'MessageSquare', template: '<span class="MessageSquare" />' },
  Server: { name: 'Server', template: '<span class="Server" />' },
  Settings: { name: 'Settings', template: '<span class="Settings" />' },
  Trash2: { name: 'Trash2', template: '<span class="Trash2" />' },
  WandSparkles: { name: 'WandSparkles', template: '<span class="WandSparkles" />' },
  Wrench: { name: 'Wrench', template: '<span class="Wrench" />' },
  X: { name: 'X', template: '<span class="X" />' },
  Zap: { name: 'Zap', template: '<span class="Zap" />' },
}));

vi.mock('../api/desktop', () => ({
  listLaputaProposals: vi.fn(() =>
    Promise.resolve([
      { id: 'proposal-1', state: 'pending_review' },
      { id: 'proposal-2', state: 'pending_review' },
      { id: 'proposal-3', state: 'pending_review' },
    ])
  ),
  pollLaputaEvents: vi.fn(() => Promise.resolve([])),
}));

vi.mock('./ChatView.vue', () => ({
  default: { name: 'ChatView', template: '<div class="chat-view-stub" />' },
}));

vi.mock('./SettingsView.vue', () => ({
  default: { name: 'SettingsView', template: '<div class="settings-view-stub" />' },
}));

vi.mock('./CronTaskManagementView.vue', () => ({
  default: { name: 'CronTaskManagementView', template: '<div class="cron-view-stub" />' },
}));

vi.mock('./ConsoleView.vue', () => ({
  default: { name: 'ConsoleView', template: '<div class="console-view-stub" />' },
}));

vi.mock('./settings/McpSettings.vue', () => ({
  default: { name: 'McpSettings', template: '<div class="mcp-settings-stub" />' },
}));

vi.mock('./settings/SkillsSettings.vue', () => ({
  default: { name: 'SkillsSettings', template: '<div class="skills-settings-stub" />' },
}));

vi.mock('./NotebookView.vue', () => ({
  default: { name: 'NotebookView', template: '<div class="notebook-view-stub" />' },
}));

vi.mock('./EvolutionView.vue', () => ({
  default: {
    name: 'EvolutionView',
    template: '<div class="evolution-view-stub" />',
    emits: ['count-change'],
    mounted() {
      this.$emit('count-change', {
        total: 3,
        tone: 'warning',
        tooltip: '3 pending reviews',
      });
    },
  },
}));

vi.mock('../features/diva-pet/components/DivaPetView.vue', () => ({
  default: {
    name: 'DivaPetView',
    template: '<div class="diva-pet-view-stub" />',
    emits: ['toggle-sidebar'],
  },
}));

vi.mock('./AppDialogLayer.vue', () => ({
  default: { name: 'AppDialogLayer', template: '<div class="dialog-layer-stub" />' },
}));

vi.mock('./AppToastLayer.vue', () => ({
  default: { name: 'AppToastLayer', template: '<div class="toast-layer-stub" />' },
}));

function mountNormalMode() {
  return mount(NormalMode, {
    props: {
      messages: [],
      isTyping: false,
      connectionStatus: 'connected',
      currentEmotion: 'happy',
      config: {
        provider: 'deepseek',
        apiBase: 'https://api.deepseek.com/v1',
        apiKey: '',
        model: 'deepseek-chat',
      },
      providerConfigs: {},
      toolsConfig: {
        web: {
          search: {
            provider: '',
            enabled: false,
            api_key: '',
            max_results: 5,
          },
          fetch: {
            enabled: false,
          },
        },
        mentle: {
          enabled: false,
          mode: 'off',
          allowed_tools: [],
        },
      },
      savedModels: [],
      sessions: [],
      chatDisplayPrefs: {
        autoExpandReasoning: false,
        autoExpandToolDetails: false,
        showRawMetaByDefault: false,
      },
      saveConfigAction: vi.fn(() => Promise.resolve()),
      saveToolsConfigAction: vi.fn(() => Promise.resolve()),
      saveChannelConfigAction: vi.fn(() => Promise.resolve()),
    },
  });
}

async function clickNav(wrapper: ReturnType<typeof mountNormalMode>, label: string) {
  const navIndexes: Record<string, number> = {
    'nav.chat': 0,
    'nav.evolution': 1,
    'nav.notebook': 2,
    'nav.pet': 3,
    'nav.console': 4,
  };
  const button = wrapper.findAll('.sidebar-nav > button.nav-item')[navIndexes[label]];
  expect(button, `nav item ${label}`).toBeTruthy();
  await button!.trigger('click');
  await nextTick();
}

describe('NormalMode pet focus layout', () => {
  it('keeps normal pages outside pet focus layout with the topbar visible', () => {
    const wrapper = mountNormalMode();

    expect(wrapper.find('.app-shell').classes()).not.toContain('pet-immersive');
    expect(wrapper.find('.topbar').exists()).toBe(true);
  });

  it('hides the topbar and collapses the sidebar on the pet page', async () => {
    const wrapper = mountNormalMode();

    await clickNav(wrapper, 'nav.pet');

    expect(wrapper.find('.app-shell').classes()).toContain('pet-immersive');
    expect(wrapper.find('.app-shell').classes()).not.toContain('sidebar-expanded');
    expect(wrapper.find('.topbar').exists()).toBe(false);
    expect(wrapper.findComponent({ name: 'DivaPetView' }).exists()).toBe(true);
  });

  it('lets the pet view toggle the sidebar without restoring the topbar', async () => {
    const wrapper = mountNormalMode();

    await clickNav(wrapper, 'nav.pet');
    wrapper.findComponent({ name: 'DivaPetView' }).vm.$emit('toggle-sidebar');
    await nextTick();

    expect(wrapper.find('aside.fixed').exists()).toBe(true);
    expect(wrapper.find('.topbar').exists()).toBe(false);

    wrapper.findComponent({ name: 'DivaPetView' }).vm.$emit('toggle-sidebar');
    await nextTick();

    expect(wrapper.find('.app-shell').classes()).not.toContain('sidebar-expanded');
    expect(wrapper.find('.topbar').exists()).toBe(false);
  });

  it('restores the normal topbar after leaving the pet page', async () => {
    const wrapper = mountNormalMode();

    await clickNav(wrapper, 'nav.pet');
    wrapper.findComponent({ name: 'DivaPetView' }).vm.$emit('toggle-sidebar');
    await nextTick();
    const chatOverlayItem = wrapper.findAll('aside.fixed button').find((button) => button.text() === 'nav.chat');
    expect(chatOverlayItem, 'overlay chat item').toBeTruthy();
    await chatOverlayItem!.trigger('click');
    await nextTick();

    expect(wrapper.find('.app-shell').classes()).not.toContain('pet-immersive');
    expect(wrapper.find('.topbar').exists()).toBe(true);
  });

  it('opens the Evolution workspace from the primary sidebar with a pending badge', async () => {
    const wrapper = mountNormalMode();
    await flushPromises();

    await clickNav(wrapper, 'nav.evolution');
    await flushPromises();

    expect(wrapper.findComponent({ name: 'EvolutionView' }).exists()).toBe(true);
    expect(wrapper.find('.evolution-nav-badge').text()).toBe('3');
  });

  it('includes Evolution in the pet overlay navigation', async () => {
    const wrapper = mountNormalMode();

    await clickNav(wrapper, 'nav.pet');
    wrapper.findComponent({ name: 'DivaPetView' }).vm.$emit('toggle-sidebar');
    await nextTick();

    const overlayItems = wrapper.findAll('aside.fixed button');
    expect(overlayItems.some((button) => button.text() === 'nav.evolution')).toBe(true);
  });
});
