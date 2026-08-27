import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import WorkspaceChip from './WorkspaceChip.vue';

const workspace = {
  root: 'C:\\Projects\\agent-diva',
  source: 'configured',
  agents_md: {
    path: 'C:\\Projects\\agent-diva\\AGENTS.md',
    digest: 'abc123',
    truncated: false,
    char_count: 42,
    present: true,
  },
};

describe('WorkspaceChip', () => {
  it.each(['process-cwd', 'legacy-default'])('labels %s as the default workspace', (source) => {
    const wrapper = mount(WorkspaceChip, {
      props: { workspace: { ...workspace, source }, state: 'ready' },
    });

    expect(wrapper.get('[data-testid="workspace-chip"]').text()).toContain('默认工作区');
    expect(wrapper.get('[data-testid="workspace-chip"]').text()).not.toContain('agent-diva');
  });

  it('identifies the active workspace and AGENTS state', async () => {
    const wrapper = mount(WorkspaceChip, {
      props: { workspace, state: 'ready' },
    });

    expect(wrapper.get('[data-testid="workspace-chip"]').text()).toContain('agent-diva');
    await wrapper.get('[data-testid="workspace-chip"]').trigger('click');
    expect(wrapper.get('[data-testid="workspace-popover"]').text()).toContain('AGENTS.md 已加载');
    expect(wrapper.get('[data-testid="workspace-popover"]').text()).toContain('C:\\Projects\\agent-diva');
  });

  it('shows the selected directory for an explicit CLI workspace', () => {
    const wrapper = mount(WorkspaceChip, {
      props: { workspace: { ...workspace, source: 'explicit-cli' }, state: 'ready' },
    });

    expect(wrapper.get('[data-testid="workspace-chip"]').text()).toContain('agent-diva');
  });

  it('opens workspace settings from the popover', async () => {
    const wrapper = mount(WorkspaceChip, {
      props: { workspace, state: 'ready' },
    });

    await wrapper.get('[data-testid="workspace-chip"]').trigger('click');
    await wrapper.get('[data-testid="workspace-popover"] button:last-child').trigger('click');

    expect(wrapper.emitted('open-settings')).toHaveLength(1);
  });

  it('opens upward when rendered in the chat footer', async () => {
    const wrapper = mount(WorkspaceChip, {
      props: { workspace, state: 'ready', placement: 'above' },
    });

    await wrapper.get('[data-testid="workspace-chip"]').trigger('click');
    expect(wrapper.get('[data-testid="workspace-popover"]').classes()).toContain('workspace-popover-above');
    expect(wrapper.get('[data-testid="workspace-popover"]').text()).toContain('切换工作区');
  });
});
