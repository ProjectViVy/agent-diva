import { createApp } from "vue";
import "./styles.css";
import DesktopMateApp from "./features/diva-mate/components/DesktopMateApp.vue";
import i18n from "./i18n";

createApp(DesktopMateApp).use(i18n as any).mount("#app");
