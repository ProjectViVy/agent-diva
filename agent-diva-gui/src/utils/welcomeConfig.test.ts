import { describe, expect, it } from 'vitest';
import {
  DEFAULT_DEEPSEEK_API_BASE,
  DEFAULT_DEEPSEEK_MODEL,
  DEFAULT_DEEPSEEK_PROVIDER,
  buildWelcomeDeepSeekConfig,
} from './welcomeConfig';

describe('buildWelcomeDeepSeekConfig', () => {
  it('writes the complete DeepSeek runtime selection from the welcome wizard key', () => {
    const config = buildWelcomeDeepSeekConfig(
      {
        provider: 'ollama',
        apiBase: 'http://localhost:11434/v1',
        apiKey: '',
        model: 'llama3',
      },
      'sk-deepseek',
    );

    expect(config).toEqual({
      provider: DEFAULT_DEEPSEEK_PROVIDER,
      apiBase: DEFAULT_DEEPSEEK_API_BASE,
      apiKey: 'sk-deepseek',
      model: DEFAULT_DEEPSEEK_MODEL,
    });
  });
});
