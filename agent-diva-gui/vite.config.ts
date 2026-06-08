import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { resolve } from "path";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [vue()],
  resolve: {
    alias: {
"avatar-runtime-vrm": resolve(__dirname, "./avatar-runtime-vrm/src/index.ts"),
      "@morediva/shared-avatar-protocol": resolve(__dirname, "./shared-avatar-protocol/src/index.ts"),
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },

  // Multi-page for desktop pet pop-out window
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        "desktop-pet": resolve(__dirname, "desktop-pet.html"),
        "embedded-pet": resolve(__dirname, "embedded-pet.html"),
      },
    },
  },
}));
