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
    include: ['src/**/*.test.ts'],
  },
})
