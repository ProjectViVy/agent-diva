export interface SavedModel {
  id: string;
  provider: string;
  model: string;
  apiBase: string;
  apiKey: string;
  displayName: string;
}

export interface ChatDisplayPrefs {
  cleanMode: boolean;
  autoExpandReasoning: boolean;
  autoExpandToolDetails: boolean;
  showRawMetaByDefault: boolean;
}

export const DEFAULT_CHAT_DISPLAY_PREFS: Readonly<ChatDisplayPrefs> = Object.freeze({
  cleanMode: false,
  autoExpandReasoning: true,
  autoExpandToolDetails: false,
  showRawMetaByDefault: false,
});
