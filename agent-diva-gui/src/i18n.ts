import { createI18n } from 'vue-i18n';
import zh from './locales/zh';
import en from './locales/en';

const savedLocale = localStorage.getItem('agent-diva-locale') || 'zh';

// Patch missing locale keys for newly added network settings in legacy locale files.
const zhPatched: any = { ...zh };
zhPatched.settings = {
  ...(zhPatched.settings || {}),
  network: zhPatched.settings?.network || '网络',
};
zhPatched.dashboard = {
  ...(zhPatched.dashboard || {}),
  network: zhPatched.dashboard?.network || '网络',
  networkDesc: zhPatched.dashboard?.networkDesc || '配置 web 搜索与抓取工具',
};
zhPatched.network = {
  ...(zhPatched.network || {}),
  title: zhPatched.network?.title || '网络工具',
  desc: zhPatched.network?.desc || '配置 web 搜索 / 抓取运行参数',
  provider: zhPatched.network?.provider || '搜索提供商',
  apiKey: zhPatched.network?.apiKey || 'Brave API Key',
  apiKeyPlaceholder: zhPatched.network?.apiKeyPlaceholder || '输入 Brave API Key...',
  maxResults: zhPatched.network?.maxResults || '最大结果数',
  enableSearch: zhPatched.network?.enableSearch || '启用 Web Search',
  enableFetch: zhPatched.network?.enableFetch || '启用 Web Fetch',
  save: zhPatched.network?.save || '保存网络设置',
};

const i18n = createI18n({
  legacy: false, // Use Composition API
  locale: savedLocale, // Default locale
  fallbackLocale: 'en',
  messages: {
    zh: zhPatched,
    en
  }
});

export default i18n;
