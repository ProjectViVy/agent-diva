import { beforeEach, describe, expect, it, vi } from 'vitest';
import { getPersonaStatus, initializePersona, savePersonaDocument } from './desktop';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

beforeEach(() => { invoke.mockReset(); invoke.mockResolvedValue({}); });

describe('Persona desktop bridge', () => {
  it('reads the independent Persona server state', async () => {
    await getPersonaStatus();
    expect(invoke).toHaveBeenCalledWith('persona_get_status');
  });

  it('forwards the five authority initialization payload verbatim', async () => {
    const payload = { identity: 'i', relationship: 'r', redline: 'x', user: 'u', world: 'raw world' };
    await initializePersona(payload);
    expect(invoke).toHaveBeenCalledWith('persona_initialize', { payload });
  });

  it('maps direct saves to the CAS payload expected by Manager', async () => {
    await savePersonaDocument('world', '# World', 4, 'user edit');
    expect(invoke).toHaveBeenCalledWith('persona_save_document', {
      kind: 'world',
      payload: { content: '# World', base_revision: 4, reason: 'user edit' },
    });
  });
});
