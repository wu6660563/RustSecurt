import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  resolve: { alias: { vue: 'vue/dist/vue.runtime.esm-bundler.js' } },
  server: { port: 1420, strictPort: true },
  test: { environment: 'jsdom' }
})
