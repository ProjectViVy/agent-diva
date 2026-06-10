import { describe, it, expect, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import { nextTick } from 'vue';
import SubAgentPanel from './SubAgentPanel.vue';

// ── Helpers ──────────────────────────────────────────────────────────────────

// Minimal i18n stub: returns the fallback string
function createI18nStub() {
  return {
    global: {
      plugins: [
        {
          install(app: any) {
            app.config.globalProperties.$t = (_key: string, fallback?: string) => fallback ?? _key;
            app.provide('vue-i18n', { t: (_k: string, f?: string) => f ?? _k });
          },
        },
      ],
    },
  };
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe('SubAgentPanel', () => {
  beforeEach(() => {
    // Reset module state if needed
  });

  it('renders empty state when no children', () => {
    const wrapper = mount(SubAgentPanel, createI18nStub());

    expect(wrapper.find('.subagent-empty').exists()).toBe(true);
    expect(wrapper.find('.subagent-empty-text').text()).toContain('暂无子代理任务');
    expect(wrapper.find('.subagent-list').exists()).toBe(false);
  });

  it('renders header with title and badge', () => {
    const wrapper = mount(SubAgentPanel, createI18nStub());

    expect(wrapper.find('.subagent-panel-title').exists()).toBe(true);
    expect(wrapper.find('.subagent-panel-title').text()).toContain('子代理');
  });

  it('shows polling dot when polling is active', () => {
    const wrapper = mount(SubAgentPanel, createI18nStub());

    // After mount, startPolling is called, so the dot should appear
    expect(wrapper.find('.subagent-polling-dot').exists()).toBe(true);
  });

  it('renders child list when children are present', async () => {
    const wrapper = mount(SubAgentPanel, createI18nStub());
    await nextTick();

    // The composable uses mock data in non-Tauri mode, so children should be populated
    const items = wrapper.findAll('.subagent-item');
    expect(items.length).toBeGreaterThan(0);
  });

  it('displays correct status icons for each status', async () => {
    const wrapper = mount(SubAgentPanel, createI18nStub());
    await nextTick();

    const icons = wrapper.findAll('.subagent-status-icon');
    const iconTexts = icons.map((icon) => icon.text());

    // Should contain all four status icons from mock data
    expect(iconTexts).toContain('✅');
    expect(iconTexts).toContain('❌');
    expect(iconTexts).toContain('⏱️');
    expect(iconTexts).toContain('🚫');
  });

  it('displays task IDs', async () => {
    const wrapper = mount(SubAgentPanel, createI18nStub());
    await nextTick();

    const taskIds = wrapper.findAll('.subagent-task-id');
    const texts = taskIds.map((el) => el.text());

    expect(texts).toContain('a1b2c3');
    expect(texts).toContain('d4e5f6');
  });

  it('displays summary text', async () => {
    const wrapper = mount(SubAgentPanel, createI18nStub());
    await nextTick();

    const summaries = wrapper.findAll('.subagent-summary');
    expect(summaries.length).toBeGreaterThan(0);
    expect(summaries[0].text()).toBeTruthy();
  });

  it('applies status-specific CSS class', async () => {
    const wrapper = mount(SubAgentPanel, createI18nStub());
    await nextTick();

    expect(wrapper.find('.subagent-status-ok').exists()).toBe(true);
    expect(wrapper.find('.subagent-status-error').exists()).toBe(true);
    expect(wrapper.find('.subagent-status-timeout').exists()).toBe(true);
    expect(wrapper.find('.subagent-status-cancelled').exists()).toBe(true);
  });

  it('renders elapsed time', async () => {
    const wrapper = mount(SubAgentPanel, createI18nStub());
    await nextTick();

    const elapsed = wrapper.findAll('.subagent-elapsed');
    expect(elapsed.length).toBeGreaterThan(0);
    // Elapsed should contain a formatted time string
    expect(elapsed[0].text()).toMatch(/\d+(ms|s|m)/);
  });

  it('shows tool call count in meta', async () => {
    const wrapper = mount(SubAgentPanel, createI18nStub());
    await nextTick();

    const meta = wrapper.findAll('.subagent-meta');
    expect(meta.length).toBeGreaterThan(0);
    expect(meta[0].text()).toContain('工具调用');
  });

  it('shows badge with total count', async () => {
    const wrapper = mount(SubAgentPanel, createI18nStub());
    await nextTick();

    const badge = wrapper.find('.subagent-badge');
    expect(badge.exists()).toBe(true);
    expect(Number(badge.text())).toBeGreaterThan(0);
  });
});
