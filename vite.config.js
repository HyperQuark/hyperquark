import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vite'
import wasm from "vite-plugin-wasm";
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'

// https://vitejs.dev/config/
export default defineConfig({
  root: './playground',
  plugins: [
    wasm(),
    vue(),
    vueJsx(),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./playground', import.meta.url))
    }
  }
})
