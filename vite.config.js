import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vite'
import wasm from "vite-plugin-wasm";
import vue from '@vitejs/plugin-vue'
import { nodePolyfills } from 'vite-plugin-node-polyfills'

// https://vitejs.dev/config/
export default defineConfig({
  root: './playground',
  base: '/hyperquark/',
  plugins: [
    wasm(),
    vue(),
    nodePolyfills({
      globals: {
        Buffer: true,
      },
    }),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./playground', import.meta.url))
    }
  },
  build: {
    target: 'esnext',
  }
})
