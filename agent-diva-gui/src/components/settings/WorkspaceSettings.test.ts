import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, beforeEach, vi } from 'vitest';
import WorkspaceSettings from './WorkspaceSettings.vue';

const workspaceApi = vi.hoisted(() => ({
  inspectWorkspace: vi.fn(),
  chooseWorkspaceDirectory: vi.fn(),
}));
const { inspectWorkspace, chooseWorkspaceDirectory } = workspaceApi;

vi.mock('../../api/desktop', () => ({
  inspectWorkspace: workspaceApi.inspectWorkspace,
  chooseWorkspaceDirectory: workspaceApi.chooseWorkspaceDirectory,
}));

vi.mock('@lucide/vue', () => {
  const icon = (name: string) => ({ name, template: `<span class="${name}" />` });
  return {
    ChevronDown: icon('ChevronDown'),
    FileText: icon('FileText'),
    FolderOpen: icon('FolderOpen'),
    RefreshCw: icon('RefreshCw'),
    ShieldCheck: icon('ShieldCheck'),
  };
});

const currentWorkspace = {
  root: 'C:\\projects\\current',
  source: 'configured',
  legacy_hint: null,
  agents_md: null,
};

function candidate(root: string) {
  return {
    root,
    workspaceId: `workspace-${root.length}`,
    readable: true,
    agentsMd: {
      path: `${root}\\AGENTS.md`,
      digest: 'abc123',
      truncated: false,
      char_count: 12,
      present: true,
    },
  };
}

function mountSettings() {
  return mount(WorkspaceSettings, {
    props: {
      workspace: currentWorkspace,
      state: 'ready',
      error: null,
      refreshWorkspace: vi.fn(() => Promise.resolve(true)),
    },
  });
}

describe('WorkspaceSettings candidate draft', () => {
  beforeEach(() => {
    inspectWorkspace.mockReset();
    chooseWorkspaceDirectory.mockReset();
  });

  it('previews a candidate without changing the current workspace', async () => {
    inspectWorkspace.mockResolvedValueOnce(candidate('C:\\projects\\next'));
    const wrapper = mountSettings();

    await wrapper.find('input').setValue('C:\\projects\\next');
    await wrapper.findAll('.workspace-settings-secondary')[1].trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('C:\\projects\\next');
    expect(wrapper.text()).toContain('候选已验证');
    expect(wrapper.text()).toContain('C:\\projects\\current');
    expect(wrapper.find('[data-testid="workspace-candidate"]').exists()).toBe(true);
  });

  it('ignores an older inspection response after a newer selection', async () => {
    let resolveFirst!: (value: ReturnType<typeof candidate>) => void;
    let resolveSecond!: (value: ReturnType<typeof candidate>) => void;
    inspectWorkspace
      .mockImplementationOnce(() => new Promise((resolve) => { resolveFirst = resolve; }))
      .mockImplementationOnce(() => new Promise((resolve) => { resolveSecond = resolve; }));

    const wrapper = mountSettings();
    const input = wrapper.find('input');
    const inspectButton = () => wrapper.findAll('.workspace-settings-secondary')[1];

    await input.setValue('C:\\projects\\first');
    await inspectButton().trigger('click');
    await input.setValue('C:\\projects\\second');
    await inspectButton().trigger('click');

    resolveSecond(candidate('C:\\projects\\second'));
    await flushPromises();
    resolveFirst(candidate('C:\\projects\\first'));
    await flushPromises();

    expect(wrapper.find('[data-testid="workspace-candidate"]').text()).toContain('C:\\projects\\second');
    expect(wrapper.find('[data-testid="workspace-candidate"]').text()).not.toContain('C:\\projects\\first');
  });

  it('only commits an inspected candidate and disables commit while blocked', async () => {
    inspectWorkspace.mockResolvedValueOnce(candidate('C:\\projects\\next'));
    const switchWorkspace = vi.fn(() => Promise.resolve(true));
    const wrapper = mount(WorkspaceSettings, {
      props: {
        workspace: currentWorkspace,
        state: 'ready',
        refreshWorkspace: vi.fn(() => Promise.resolve(true)),
        switchWorkspace,
        switchBlockedReason: '当前仍有流式输出，请先停止。',
      },
    });

    await wrapper.find('input').setValue('C:\\projects\\next');
    await wrapper.findAll('.workspace-settings-secondary')[1].trigger('click');
    await flushPromises();

    const commitButton = wrapper.find('.workspace-settings-primary');
    expect(commitButton.attributes('disabled')).toBeDefined();
    expect(switchWorkspace).not.toHaveBeenCalled();
  });
});
