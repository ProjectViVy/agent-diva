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
    is_gateway: false,
    is_local: false,
    default_api_base: 'https://api.openai.com/v1',
    models: ['gpt-4o', 'gpt-4-turbo', 'gpt-3.5-turbo'],
  },
  {
    name: 'anthropic',
    api_type: 'anthropic',
    keywords: ['anthropic'],
    env_key: 'ANTHROPIC_API_KEY',
    display_name: 'Anthropic',
    litellm_prefix: 'anthropic',
    skip_prefixes: [],
    is_gateway: false,
    is_local: false,
    default_api_base: 'https://api.anthropic.com/v1',
    models: ['claude-3-5-sonnet-20240620', 'claude-3-opus-20240229'],
  },
  {
    name: 'ollama',
    api_type: 'ollama',
    keywords: ['ollama'],
    env_key: 'OLLAMA_API_BASE',
    display_name: 'Ollama',
    litellm_prefix: 'ollama',
    skip_prefixes: [],
    is_gateway: false,
    is_local: true,
    default_api_base: 'http://localhost:11434',
    models: ['llama3', 'mistral', 'qwen2'],
  },
];

const mockChannels = {
  telegram: { enabled: true, token: '123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11' },
  discord: { enabled: false, token: '', allow_from: [] },
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
      apiBase: 'https://api.openai.com/v1',
      apiKey: 'sk-proj-1234567890abcdef1234567890abcdef',
      model: 'gpt-4o',
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
      apiBase: 'http://localhost:11434',
      apiKey: '',
      model: 'llama3',
    },
    savedModels: [],
  },
};
