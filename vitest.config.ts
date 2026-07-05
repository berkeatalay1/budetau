import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'

// Unit-test config, separate from vite.config.ts (which carries Tauri-specific
// dev-server options). Tests live next to the code as `*.spec.ts`.
export default defineConfig({
  plugins: [vue()],
  test: {
    environment: 'jsdom',
    include: ['src/**/*.spec.ts'],
  },
})
