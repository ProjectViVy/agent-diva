import { createI18n } from 'vue-i18n';
import zh from './locales/zh';
import en from './locales/en';

const savedLocale = localStorage.getItem('agent-diva-locale') || 'zh';

const i18n = createI18n({
  legacy: false, // Use Composition API
  locale: savedLocale, // Default locale
  fallbackLocale: 'en',
  messages: {
    zh,
    en
  }
});

export default i18n;
