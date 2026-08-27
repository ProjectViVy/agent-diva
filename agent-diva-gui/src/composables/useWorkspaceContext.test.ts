import { beforeEach, describe, expect, it, vi } from 'vitest';

const workspaceApi = vi.hoisted(() => ({
  getWorkspaceStatus: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

vi.mock('../api/desktop', () => workspaceApi);

import { useWorkspaceContext } from './useWorkspaceContext';

const defaultStatus = {
  root: 'C:\\Users\\Administrator\\Pictures',
  source: 'configured',
  uses_default_workspace: true,
  legacy_hint: null,
  agents_md: null,
};

describe('useWorkspaceContext', () => {
  beforeEach(() => {
    workspaceApi.getWorkspaceStatus.mockReset();
    workspaceApi.isTauriRuntime.mockReturnValue(true);
    useWorkspaceContext().apply(defaultStatus);
  });

  it('keeps an explicit same-root session selection after a status refresh', async () => {
    const context = useWorkspaceContext();
    context.apply({
      ...defaultStatus,
      source: 'explicit-cli',
      uses_default_workspace: false,
    });
    workspaceApi.getWorkspaceStatus.mockResolvedValue(defaultStatus);

    expect(await context.refresh()).toBe(true);
    expect(context.status.value?.source).toBe('configured');
    expect(context.status.value?.uses_default_workspace).toBe(false);
  });

  it('keeps a configured runtime directory-labeled after the default is reset', async () => {
    const detachedStatus = {
      ...defaultStatus,
      uses_default_workspace: false,
    };
    const context = useWorkspaceContext();
    context.apply(detachedStatus);
    workspaceApi.getWorkspaceStatus.mockResolvedValue(detachedStatus);

    expect(await context.refresh()).toBe(true);
    expect(context.status.value?.source).toBe('configured');
    expect(context.status.value?.uses_default_workspace).toBe(false);
  });
});
