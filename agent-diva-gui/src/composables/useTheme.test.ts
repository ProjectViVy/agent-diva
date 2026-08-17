import { beforeEach, describe, expect, it, vi } from 'vitest';

async function freshModule() {
  vi.resetModules();
  return import('./useTheme');
}

describe('useTheme', () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
  });

  it('defaults to love and applies data-theme on load', async () => {
    const { useTheme } = await freshModule();
    const { theme } = useTheme();
    expect(theme.value).toBe('love');
    expect(document.documentElement.getAttribute('data-theme')).toBe('love');
  });

  it('applies and persists theme changes', async () => {
    const { useTheme } = await freshModule();
    const { theme, setTheme } = useTheme();
    setTheme('dark');
    expect(theme.value).toBe('dark');
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    expect(localStorage.getItem('agent-diva-theme')).toBe('dark');
  });

  it('ignores unknown theme ids', async () => {
    const { useTheme } = await freshModule();
    const { theme, setTheme } = useTheme();
    setTheme('neon');
    expect(theme.value).toBe('love');
    expect(document.documentElement.getAttribute('data-theme')).toBe('love');
  });

  it('restores persisted theme on load', async () => {
    localStorage.setItem('agent-diva-theme', 'miku');
    const { useTheme } = await freshModule();
    const { theme } = useTheme();
    expect(theme.value).toBe('miku');
    expect(document.documentElement.getAttribute('data-theme')).toBe('miku');
  });

  it('falls back to default when stored value is invalid', async () => {
    localStorage.setItem('agent-diva-theme', 'bogus');
    const { useTheme } = await freshModule();
    const { theme } = useTheme();
    expect(theme.value).toBe('love');
  });
});
