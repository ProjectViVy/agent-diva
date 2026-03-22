/** Keys and helpers for Agent Diva GUI WebView local storage. */

export const STORAGE_PREFIX = "agent-diva-";

export const WELCOME_STORAGE_KEY = "agent-diva-welcome-v1";
export const HISTORY_PREFS_KEY = "agent-diva-history-prefs";
export const SESSION_CACHE_PREFIX = "agent-diva-session-cache:";
export const SAVED_MODELS_KEY = "agent-diva-saved-models";
export const LOCALE_STORAGE_KEY = "agent-diva-locale";

export const UI_CACHE_KEYS = [SAVED_MODELS_KEY, HISTORY_PREFS_KEY] as const;
export const UI_CACHE_PREFIXES = [SESSION_CACHE_PREFIX] as const;

/** Removes every `agent-diva-*` key (session cache keys use the same prefix). */
export function clearAgentDivaLocalStorage(options?: {
  preserveLocale?: boolean;
}): void {
  if (typeof localStorage === "undefined") {
    return;
  }
  const keys = Object.keys(localStorage);
  for (const key of keys) {
    if (!key.startsWith(STORAGE_PREFIX)) {
      continue;
    }
    if (options?.preserveLocale && key === LOCALE_STORAGE_KEY) {
      continue;
    }
    localStorage.removeItem(key);
  }
}
