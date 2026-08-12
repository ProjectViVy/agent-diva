import { describe, expect, it } from 'vitest';
import { errorMessage } from './errorMessage';

describe('errorMessage', () => {
  it('extracts structured Tauri errors without rendering object Object', () => {
    expect(errorMessage({ message: 'approval request not found', status: 404 })).toBe(
      'approval request not found',
    );
    expect(errorMessage({ error: 'governance ledger failed' })).toBe('governance ledger failed');
  });

  it('uses useful serialization and a stable fallback', () => {
    expect(errorMessage({ code: 'BROKEN' })).toBe('{"code":"BROKEN"}');
    expect(errorMessage(null, 'Load failed')).toBe('Load failed');
  });
});
