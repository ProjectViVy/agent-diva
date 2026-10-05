import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';
import NormalMode from './NormalMode.vue';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('@lucide/vue', () => ({
  AlarmClock: { name: 'AlarmClock', template: '<span class="AlarmClock" />' },
  BookOpen: { name: 'BookOpen', template: '<span class="BookOpen" />' },
  BookUser: { name: 'BookUser', template: '<span class="BookUser" />' },
  Bot: { name: 'Bot', template: '<span class="Bot" />' },
  Brain: { name: 'Brain', template: '<span class="Brain" />' },
  Cat: { name: 'Cat', template: '<span class="Cat" />' },
  Check: { name: 'Check', template: '<span class="Check" />' },
  ChevronDown: { name: 'ChevronDown', template: '<span class="ChevronDown" />' },
  ClipboardList: { name: 'ClipboardList', template: '<span class="ClipboardList" />' },
  Database: { name: 'Database', template: '<span class="Database" />' },
  GitBranch: { name: 'GitBranch', template: '<span class="GitBranch" />' },
  Heart: { name: 'Heart', template: '<span class="Heart" />' },
  Menu: { name: 'Menu', template: '<span class="Menu" />' },
  MessageSquare: { name: 'MessageSquare', template: '<span class="MessageSquare" />' },
  Server: { name: 'Server', template: '<span class="Server" />' },
  Settings: { name: 'Settings', template: '<span class="Settings" />' },
  Sparkles: { name: 'Sparkles', template: '<span class="Sparkles" />' },
  Trash2: { name: 'Trash2', template: '<span class="Trash2" />' },
  WandSparkles: { name: 'WandSparkles', template: '<span class="WandSparkles" />' },
  Wrench: { name: 'Wrench', template: '<span class="Wrench" />' },
  X: { name: 'X', template: '<span class="X" />' },
  Zap: { name: 'Zap', template: '<span class="Zap" />' },
}));

vi.mock('../api/desktop', () => ({
  isTauriRuntime: () => false,
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

function mountNormalMode(propOverrides: Record<string, unknown> = {}) {
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
      savedModels: [],
      sessions: [],
      chatDisplayPrefs: {
        cleanMode: false,
        autoExpandReasoning: false,
        autoExpandToolDetails: false,
        showRawMetaByDefault: false,
      },
      saveConfigAction: vi.fn(() => Promise.resolve()),
      ...propOverrides,
    },
  });
}

async function clickNav(wrapper: ReturnType<typeof mountNormalMode>, label: string) {
  const sidebar = wrapper.find('.sidebar');
  if (sidebar.classes().includes('sidebar-collapsed')) {
    const toggle = wrapper.find('.menu-toggle');
    expect(toggle.exists(), 'menu toggle').toBe(true);
    await toggle.trigger('click');
    await nextTick();
  }

  const buttons = wrapper.findAll('.sidebar-nav button');
  const target = buttons.find((button) => button.text().includes(label));
  expect(target, `nav item ${label}`).toBeTruthy();
  await target!.trigger('click');
  await nextTick();
}

describe('NormalMode pet focus layout', () => {
  it('exposes the tools group as a button and reports its expanded state in both sidebar modes', async () => {
    const wrapper = mountNormalMode();
    let group = wrapper.get('.nav-group-header');

    expect(group.element.tagName).toBe('BUTTON');
    expect(group.attributes('type')).toBe('button');
    expect(group.attributes('aria-label')).toBe('nav.toolsGroup');
    expect(group.attributes('aria-expanded')).toBe('false');

    await group.trigger('click');
    await nextTick();
    expect(group.attributes('aria-expanded')).toBe('true');
    expect(wrapper.findAll('.popup-menu-item')).toHaveLength(2);

    await group.trigger('click');
    await nextTick();
    expect(group.attributes('aria-expanded')).toBe('false');

    await wrapper.find('.menu-toggle').trigger('click');
    await nextTick();
    expect(group.attributes('aria-expanded')).toBe('true');

    await group.trigger('click');
    await nextTick();
    expect(group.attributes('aria-expanded')).toBe('false');
    expect(wrapper.find('.nav-group-items').element.getAttribute('style')).toContain('display: none');
  });

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
    expect(wrapper.find('.sidebar').exists()).toBe(false);
    expect(wrapper.find('.main-panel').exists()).toBe(true);
    expect(wrapper.findComponent({ name: 'DivaPetView' }).exists()).toBe(true);
  });

  it('lets the pet view toggle the sidebar without restoring the topbar', async () => {
    const wrapper = mountNormalMode();

    await clickNav(wrapper, 'nav.pet');
    wrapper.findComponent({ name: 'DivaPetView' }).vm.$emit('toggle-sidebar');
    await nextTick();

    expect(wrapper.find('.overlay-sidebar').exists()).toBe(true);
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
    const chatOverlayItem = wrapper.findAll('.overlay-sidebar button.nav-item').find((button) => button.text() === 'nav.chat');
    expect(chatOverlayItem, 'overlay chat item').toBeTruthy();
    await chatOverlayItem!.trigger('click');
    await nextTick();

    expect(wrapper.find('.app-shell').classes()).not.toContain('pet-immersive');
    expect(wrapper.find('.topbar').exists()).toBe(true);
  });

});

// describe('NormalMode backend disconnected indicator', () => {
//   it('hides the disconnected indicator when the backend is connected', () => {
//     const wrapper = mountNormalMode({ connectionStatus: 'connected' });
//
//     expect(wrapper.find('[data-testid="backend-disconnected-indicator"]').exists()).toBe(false);
//   });
//
//   it('shows the disconnected indicator with tooltip when the backend is unavailable', () => {
//     const wrapper = mountNormalMode({ connectionStatus: 'error' });
//     const indicator = wrapper.get('[data-testid="backend-disconnected-indicator"]');
//
//     expect(indicator.attributes('title')).toBe('app.backendDisconnected');
//     expect(indicator.attributes('aria-label')).toBe('app.backendDisconnected');
//     expect(indicator.find('.TriangleAlert').exists()).toBe(true);
//   });
//
//   it('keeps the model dropdown working while the disconnected indicator is visible', async () => {
//     const wrapper = mountNormalMode({
//       connectionStatus: 'error',
//       savedModels: [
//         {
//           id: 'deepseek:deepseek-chat',
//           provider: 'deepseek',
//           model: 'deepseek-chat',
//           apiBase: 'https://api.deepseek.com/v1',
//           apiKey: '',
//           displayName: 'DeepSeek - deepseek-chat',
//         },
//       ],
//     });
//
//     const modelButton = wrapper.get('.topbar-right button');
//     await modelButton.trigger('click');
//     await nextTick();
//
//     expect(wrapper.find('[data-testid="backend-disconnected-indicator"]').exists()).toBe(true);
//     expect(wrapper.text()).toContain('DeepSeek - deepseek-chat');
//   });
// });
