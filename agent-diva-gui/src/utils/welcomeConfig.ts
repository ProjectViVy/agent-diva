export const DEFAULT_DEEPSEEK_PROVIDER = 'deepseek';
export const DEFAULT_DEEPSEEK_API_BASE = 'https://api.deepseek.com/v1';
export const DEFAULT_DEEPSEEK_MODEL = 'deepseek-chat';

export interface AppProviderConfig {
  provider: string;
  apiBase: string;
  apiKey: string;
  model: string;
}

export function buildWelcomeDeepSeekConfig(
  currentConfig: AppProviderConfig,
  apiKey: string,
): AppProviderConfig {
  return {
    ...currentConfig,
    provider: DEFAULT_DEEPSEEK_PROVIDER,
    apiBase: DEFAULT_DEEPSEEK_API_BASE,
    apiKey,
    model: DEFAULT_DEEPSEEK_MODEL,
  };
}
