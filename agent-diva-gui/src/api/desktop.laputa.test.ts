import { describe, it, expect, vi, beforeEach } from 'vitest';
import { getLaputaSnapshot, writeLaputaSection } from './desktop';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

beforeEach(() => {
  invoke.mockReset();
  invoke.mockResolvedValue({});
});

describe('getLaputaSnapshot', () => {
  it('calls laputa_get_snapshot with null when since is omitted', async () => {
    await getLaputaSnapshot();
    expect(invoke).toHaveBeenCalledWith('laputa_get_snapshot', { since: null });
  });

  it('forwards the since parameter', async () => {
    await getLaputaSnapshot('2026-07-05T00:00:00Z');
    expect(invoke).toHaveBeenCalledWith('laputa_get_snapshot', { since: '2026-07-05T00:00:00Z' });
  });
});

describe('writeLaputaSection', () => {
  it('calls laputa_write_section with name, content and null summary', async () => {
    invoke.mockResolvedValue({
      proposal_id: 'proposal-1',
      proposal_type: 'identity_patch',
      risk_level: 'high',
      state: 'pending_review',
    });
    await writeLaputaSection('identity', '# Identity\n\nHello', undefined);
    expect(invoke).toHaveBeenCalledWith('laputa_write_section', {
      name: 'identity',
      content: '# Identity\n\nHello',
      summary: null,
    });
  });

  it('forwards the optional summary', async () => {
    invoke.mockResolvedValue({
      proposal_id: 'proposal-2',
      proposal_type: 'memory_patch',
      risk_level: 'medium',
      state: 'pending_review',
    });
    await writeLaputaSection('memory_md', '{"note":"x"}', 'Quick edit');
    expect(invoke).toHaveBeenCalledWith('laputa_write_section', {
      name: 'memory_md',
      content: '{"note":"x"}',
      summary: 'Quick edit',
    });
  });
});
