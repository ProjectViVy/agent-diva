import { createApp } from "vue";
import "./styles.css";
import App from "./App.vue";
import i18n from "./i18n";
import { installGuiLogger } from './utils/guiLogger';

installGuiLogger('main');
const app = createApp(App);
app.config.errorHandler = (error, _instance, info) => {
  console.error('vue.unhandled-error', { error, info });
};
app.use(i18n as any).mount("#app");
