import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath, URL } from 'node:url';

const host = process.env.TAURI_DEV_HOST;
const giphyApiKey = process.env.MIXIN_GIPHY_KEY ?? '';

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      '@flutter-assets': fileURLToPath(new URL('../flutter/assets', import.meta.url)),
    },
  },
  define: {
    'import.meta.env.MIXIN_GIPHY_KEY': JSON.stringify(giphyApiKey),
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
});
