import { createApp } from "vue";
import "./styles.css";
import DesktopPetApp from "./features/diva-pet/components/DesktopPetApp.vue";
import i18n from "./i18n";

createApp(DesktopPetApp).use(i18n as any).mount("#app");
