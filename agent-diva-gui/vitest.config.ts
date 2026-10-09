import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      'avatar-runtime-vrm': resolve(__dirname, './avatar-runtime-vrm/src/index.ts'),
      '@morediva/shared-avatar-protocol': resolve(__dirname, './shared-avatar-protocol/src/index.ts'),
    },
  },
  test: {
    environment: 'happy-dom',
    globals: true,
    css: true,
    // Wails runtime starts a short drag-init interval at import time. Under
    // parallel happy-dom worker teardown, the callback can run after its
    // window is gone; serial files let that runtime timer settle in-scope.
    maxWorkers: 1,
    include: ['src/**/*.{test,spec}.ts'],
  },
})
