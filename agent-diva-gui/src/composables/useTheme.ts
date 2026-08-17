import { readonly, ref } from 'vue';

export const THEME_IDS = ['love', 'dark', 'default', 'miku'] as const;
export type ThemeId = (typeof THEME_IDS)[number];

const STORAGE_KEY = 'agent-diva-theme';
const DEFAULT_THEME: ThemeId = 'love';

function isThemeId(value: string | null | undefined): value is ThemeId {
  return !!value && (THEME_IDS as readonly string[]).includes(value);
}

function loadStoredTheme(): ThemeId {
  try {
    const saved = window.localStorage.getItem(STORAGE_KEY);
    return isThemeId(saved) ? saved : DEFAULT_THEME;
  } catch {
    return DEFAULT_THEME;
  }
}

const theme = ref<ThemeId>(loadStoredTheme());

function applyTheme(value: ThemeId) {
  if (typeof document === 'undefined') return;
  document.documentElement.setAttribute('data-theme', value);
}

applyTheme(theme.value);

/** 单一主题入口：状态、data-theme 应用、localStorage 持久化。 */
export function useTheme() {
  const setTheme = (value: string) => {
    if (!isThemeId(value)) return;
    theme.value = value;
    applyTheme(value);
    try {
      window.localStorage.setItem(STORAGE_KEY, value);
    } catch {
      /* storage unavailable (e.g. tests/SSR) — theme still applied */
    }
  };

  return { theme: readonly(theme), setTheme };
}
