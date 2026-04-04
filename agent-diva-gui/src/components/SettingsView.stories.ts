import type { Meta, StoryObj } from '@storybook/vue3';
import SettingsView from './SettingsView.vue';

const meta = {
  title: 'Components/SettingsView',
  component: SettingsView,
  tags: ['autodocs'],
  argTypes: {
    onSave: { action: 'save' },
    'onUpdate-saved-models': { action: 'update-saved-models' },
  },
} satisfies Meta<typeof SettingsView>;

export default meta;
type Story = StoryObj<typeof meta>;

const mockProviders = [
  {
    name: 'openai',
    api_type: 'openai',
    keywords: ['openai'],
    env_key: 'OPENAI_API_KEY',
    display_name: 'OpenAI',
    litellm_prefix: 'openai',
    skip_prefixes: [],
    default_api_base: 'https://api.openai.com/v1',
    models: ['gpt-4o', 'gpt-4-turbo', 'gpt-3.5-turbo'],
    custom_models: ['gpt-4.1-mini'],
  },
  {
    name: 'anthropic',
    api_type: 'anthropic',
    keywords: ['anthropic'],
    env_key: 'ANTHROPIC_API_KEY',
    display_name: 'Anthropic',
    litellm_prefix: 'anthropic',
    skip_prefixes: [],
    default_api_base: 'https://api.anthropic.com/v1',
    models: ['claude-3-5-sonnet-20240620', 'claude-3-opus-20240229'],
    custom_models: [],
  },
  {
    name: 'ollama',
    api_type: 'ollama',
    keywords: ['ollama'],
    env_key: 'OLLAMA_API_BASE',
    display_name: 'Ollama',
    litellm_prefix: 'ollama',
    skip_prefixes: [],
    default_api_base: 'http://localhost:11434',
    models: ['llama3', 'mistral', 'qwen2'],
    custom_models: [],
  },
];

const mockChannels = {
  telegram: { enabled: true, token: '123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11' },
  discord: {
    enabled: false,
    token: '',
    allow_from: [],
    gateway_url: 'wss://gateway.discord.gg/?v=10&encoding=json',
    intents: 37377,
    guild_id: null,
    mention_only: false,
    listen_to_bots: false,
    group_reply_allowed_sender_ids: [],
  },
  dingtalk: { enabled: true, client_id: 'ding123', client_secret: 'sec456' },
  feishu: { enabled: false, app_id: '', app_secret: '', verification_token: '' },
  whatsapp: { enabled: false, bridge_url: '' },
  email: { enabled: false, imap_host: '', imap_username: '' },
  slack: { enabled: false, bot_token: '', app_token: '' },
  qq: { enabled: false, app_id: '', secret: '' },
};

// Mock invoke for Storybook
(window as any).__TAURI_INTERNALS__ = {
  invoke: async (cmd: string, args: any) => {
    if (cmd === 'get_providers') return mockProviders;
    if (cmd === 'create_custom_provider') {
      return {
        name: args.payload.id,
        api_type: 'openai',
        source: 'custom',
        configured: true,
        ready: true,
        display_name: args.payload.displayName,
        default_api_base: args.payload.apiBase,
        default_model: args.payload.defaultModel,
        models: args.payload.models || [],
        custom_models: args.payload.models || [],
      };
    }
    if (cmd === 'delete_custom_provider') {
      return;
    }
    if (cmd === 'add_provider_model') {
      return {
        provider: args.provider,
        source: 'static_fallback',
        runtime_supported: true,
        api_base: 'https://api.example.com/v1',
        models: ['gpt-4o', 'gpt-4o-mini', 'gpt-5-mini', args.model],
        custom_models: [args.model],
        warnings: [],
        error: null,
      };
    }
    if (cmd === 'remove_provider_model') {
      return {
        provider: args.provider,
        source: 'static_fallback',
        runtime_supported: true,
        api_base: 'https://api.example.com/v1',
        models: ['gpt-4o', 'gpt-4o-mini', 'gpt-5-mini'],
        custom_models: [],
        warnings: [],
        error: null,
      };
    }
    if (cmd === 'get_provider_models') {
      return {
        provider: args.provider,
        source: 'runtime',
        runtime_supported: true,
        api_base: 'https://api.example.com/v1',
        models: ['gpt-4o', 'gpt-4o-mini', 'gpt-5-mini', 'gpt-4.1-mini'],
        custom_models: ['gpt-4.1-mini'],
        warnings: [],
        error: null,
      };
    }
    if (cmd === 'test_provider_model') {
      await new Promise((resolve) => setTimeout(resolve, 600));
      return {
        ok: args.model !== 'gpt-4-turbo',
        message:
          args.model !== 'gpt-4-turbo'
            ? 'Connection test succeeded.'
            : 'Connection test failed: upstream timeout',
        latency_ms: 612,
      };
    }
    if (cmd === 'get_channels') return mockChannels;
    if (cmd === 'update_channel') {
      console.log('Invoke update_channel', args);
      return;
    }
    return null;
  },
};

// Mock fetch for Storybook
const originalFetch = window.fetch;
window.fetch = async (input, init) => {
    const url = input.toString();
    if (url.includes('/test')) {
        await new Promise(r => setTimeout(r, 1000));
        if (url.includes('dingtalk')) return new Response(JSON.stringify({ success: true }));
        return new Response(JSON.stringify({ success: false, message: 'Connection timed out' }), { status: 400 });
    }
    if (url.includes('/config')) {
        await new Promise(r => setTimeout(r, 800));
        return new Response(JSON.stringify({ success: true }));
    }
    return originalFetch(input, init);
};

export const Default: Story = {
  args: {
    config: {
      provider: 'openai',
      apiBase: 'https://api.openai.com/v1',
      apiKey: 'sk-proj-1234567890abcdef1234567890abcdef',
      model: 'gpt-4o',
    },
    toolsConfig: {
      web: {
        search: {
          provider: 'bocha',
          enabled: true,
          api_key: '',
          max_results: 5,
        },
        fetch: {
          enabled: true,
        },
      },
    },
    savedModels: [
      {
        id: 'openai:gpt-4o',
        provider: 'openai',
        model: 'gpt-4o',
        apiBase: 'https://api.openai.com/v1',
        apiKey: 'sk-proj-1234567890abcdef1234567890abcdef',
        displayName: 'OpenAI - gpt-4o',
      },
    ],
  },
};

export const LocalProvider: Story = {
  args: {
    config: {
      provider: 'ollama',
      apiBase: 'http://localhost:11434',
      apiKey: '',
      model: 'llama3',
    },
    toolsConfig: {
      web: {
        search: {
          provider: 'bocha',
          enabled: true,
          api_key: '',
          max_results: 5,
        },
        fetch: {
          enabled: true,
        },
      },
    },
    savedModels: [],
  },
};
