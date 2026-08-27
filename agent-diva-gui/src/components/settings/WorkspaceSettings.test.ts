import { flushPromises, mount } from '@vue/test-utils';
import { describe, expect, it, beforeEach, vi } from 'vitest';
import WorkspaceSettings from './WorkspaceSettings.vue';

const workspaceApi = vi.hoisted(() => ({
  inspectWorkspace: vi.fn(),
  chooseWorkspaceDirectory: vi.fn(),
  getDefaultWorkspace: vi.fn(),
  setDefaultWorkspace: vi.fn(),
  resetDefaultWorkspace: vi.fn(),
}));
const {
  inspectWorkspace,
  chooseWorkspaceDirectory,
  getDefaultWorkspace,
  setDefaultWorkspace,
  resetDefaultWorkspace,
} = workspaceApi;

vi.mock('../../api/desktop', () => ({
  inspectWorkspace: workspaceApi.inspectWorkspace,
  chooseWorkspaceDirectory: workspaceApi.chooseWorkspaceDirectory,
  getDefaultWorkspace: workspaceApi.getDefaultWorkspace,
  setDefaultWorkspace: workspaceApi.setDefaultWorkspace,
  resetDefaultWorkspace: workspaceApi.resetDefaultWorkspace,
}));

vi.mock('@lucide/vue', () => {
  const icon = (name: string) => ({ name, template: `<span class="${name}" />` });
  return {
    ChevronDown: icon('ChevronDown'),
    FileText: icon('FileText'),
    FolderOpen: icon('FolderOpen'),
    RefreshCw: icon('RefreshCw'),
    RotateCcw: icon('RotateCcw'),
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
    getDefaultWorkspace.mockReset();
    setDefaultWorkspace.mockReset();
    resetDefaultWorkspace.mockReset();
    getDefaultWorkspace.mockResolvedValue({
      root: 'C:\\projects\\default',
      isBuiltInDefault: false,
    });
  });

  it('previews a candidate without changing the current workspace', async () => {
    inspectWorkspace.mockResolvedValueOnce(candidate('C:\\projects\\next'));
    const wrapper = mountSettings();
    await flushPromises();

    await wrapper.find('input').setValue('C:\\projects\\next');
    await wrapper.findAll('.workspace-settings-secondary')[1].trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('C:\\projects\\next');
    expect(wrapper.text()).toContain('候选已验证');
    expect(wrapper.text()).toContain('C:\\projects\\current');
    expect(wrapper.find('[data-testid="workspace-candidate"]').exists()).toBe(true);
  });

  it('shows canonical Windows paths without the verbatim prefix', async () => {
    const verbatimRoot = '\\\\?\\C:\\Users\\Administrator\\Pictures';
    inspectWorkspace.mockResolvedValueOnce(candidate(verbatimRoot));
    const wrapper = mount(WorkspaceSettings, {
      props: {
        workspace: { ...currentWorkspace, root: verbatimRoot },
        state: 'ready',
        refreshWorkspace: vi.fn(() => Promise.resolve(true)),
      },
    });
    await flushPromises();

    await wrapper.find('input').setValue('C:\\Users\\Administrator\\Pictures');
    await wrapper.findAll('.workspace-settings-secondary')[1].trigger('click');
    await flushPromises();

    expect(wrapper.text()).toContain('C:\\Users\\Administrator\\Pictures');
    expect(wrapper.text()).not.toContain('\\\\?\\');
  });

  it('ignores an older inspection response after a newer selection', async () => {
    let resolveFirst!: (value: ReturnType<typeof candidate>) => void;
    let resolveSecond!: (value: ReturnType<typeof candidate>) => void;
    inspectWorkspace
      .mockImplementationOnce(() => new Promise((resolve) => { resolveFirst = resolve; }))
      .mockImplementationOnce(() => new Promise((resolve) => { resolveSecond = resolve; }));

    const wrapper = mountSettings();
    await flushPromises();
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

  it('saves an inspected candidate as the default without switching the session', async () => {
    inspectWorkspace.mockResolvedValueOnce(candidate('C:\\projects\\next'));
    setDefaultWorkspace.mockResolvedValueOnce({
      root: 'C:\\projects\\next',
      isBuiltInDefault: false,
    });
    const wrapper = mountSettings();
    await flushPromises();

    await wrapper.find('input').setValue('C:\\projects\\next');
    await wrapper.findAll('.workspace-settings-secondary')[1].trigger('click');
    await flushPromises();

    const commitButton = wrapper.find('.workspace-settings-primary');
    await commitButton.trigger('click');
    await flushPromises();

    expect(setDefaultWorkspace).toHaveBeenCalledWith('C:\\projects\\next');
    expect(wrapper.text()).toContain('C:\\projects\\current');
  });

  it('resets only the persisted default workspace', async () => {
    resetDefaultWorkspace.mockResolvedValueOnce({
      root: 'C:\\Users\\Administrator\\.agent-diva\\workspace',
      isBuiltInDefault: true,
    });
    const wrapper = mountSettings();
    await flushPromises();

    expect(wrapper.text()).toContain('默认工作区目录');
    await wrapper.get('[data-testid="workspace-reset"]').trigger('click');
    await flushPromises();

    expect(resetDefaultWorkspace).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain('C:\\projects\\current');
    expect(wrapper.text()).toContain('Diva 内置默认目录');
  });
});
