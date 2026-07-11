/** Keys and helpers for Agent Diva GUI WebView local storage. */

export const STORAGE_PREFIX = "agent-diva-";

export const WELCOME_STORAGE_KEY = "agent-diva-welcome-v1";
export const HISTORY_PREFS_KEY = "agent-diva-history-prefs";
export const SESSION_CACHE_PREFIX = "agent-diva-session-cache:";
export const SAVED_MODELS_KEY = "agent-diva-saved-models";
export const LOCALE_STORAGE_KEY = "agent-diva-locale";
export const GUI_PREFS_KEY = "agent-diva-gui-prefs";

/** Session history cache TTL used by the GUI load path. */
export const SESSION_CACHE_TTL_MS = 30 * 60 * 1000;

export interface GuiPrefs {
  closeToTray: boolean;
}

export const defaultGuiPrefs: GuiPrefs = {
  closeToTray: false,
};

export const UI_CACHE_KEYS = [SAVED_MODELS_KEY, HISTORY_PREFS_KEY] as const;
export const UI_CACHE_PREFIXES = [SESSION_CACHE_PREFIX] as const;

/**
 * localStorage keys that may hold a session snapshot for the given chat/session id.
 * Accepts either `gui:chat-…` or bare chat id.
 */
export function sessionCacheStorageKeys(chatIdOrSessionKey: string): string[] {
  if (!chatIdOrSessionKey) return [];
  const normalized = chatIdOrSessionKey.includes(":")
    ? chatIdOrSessionKey
    : `gui:${chatIdOrSessionKey}`;
  const ids = [normalized];
  if (chatIdOrSessionKey !== normalized) {
    ids.push(chatIdOrSessionKey);
  }
  return ids.map((id) => `${SESSION_CACHE_PREFIX}${id}`);
}

/** Drop all localStorage session-cache entries for the given ids. */
export function invalidateSessionCache(...chatIdOrSessionKeys: string[]): void {
  if (typeof localStorage === "undefined") {
    return;
  }
  const keys = new Set<string>();
  for (const id of chatIdOrSessionKeys) {
    for (const key of sessionCacheStorageKeys(id)) {
      keys.add(key);
    }
  }
  for (const key of keys) {
    try {
      localStorage.removeItem(key);
    } catch {
      // ignore quota / privacy mode failures
    }
  }
}

/**
 * True when a cache fallback is likely behind the sidebar list metadata.
 * `listedMessageCount` is the backend-visible user/assistant/tool count.
 */
export function isSessionCacheStaleAgainstList(
  cachedVisibleMessageCount: number,
  listedMessageCount: number | undefined | null,
): boolean {
  if (listedMessageCount == null || !Number.isFinite(listedMessageCount)) {
    return false;
  }
  return listedMessageCount > cachedVisibleMessageCount;
}

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
