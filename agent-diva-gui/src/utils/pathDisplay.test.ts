import { describe, expect, it } from 'vitest';
import { formatDisplayPath } from './pathDisplay';

describe('formatDisplayPath', () => {
  it('removes the Windows verbatim prefix from drive paths', () => {
    expect(formatDisplayPath('\\\\?\\C:\\Users\\Administrator\\Pictures'))
      .toBe('C:\\Users\\Administrator\\Pictures');
  });

  it('converts verbatim UNC paths back to their familiar form', () => {
    expect(formatDisplayPath('\\\\?\\UNC\\server\\share\\workspace'))
      .toBe('\\\\server\\share\\workspace');
  });

  it('keeps ordinary Windows and POSIX paths unchanged', () => {
    expect(formatDisplayPath('C:\\Projects\\agent-diva')).toBe('C:\\Projects\\agent-diva');
    expect(formatDisplayPath('/home/diva/workspace')).toBe('/home/diva/workspace');
  });
});
