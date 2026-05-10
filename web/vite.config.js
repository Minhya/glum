import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  server: {
    host: 'localhost',
    port: 4000,
    strictPort: true,
    proxy: {
      '/auth': 'http://localhost:8090',
      '/glum.': 'http://localhost:8090',
    },
  },
  preview: {
    host: 'localhost',
    port: 4000,
    strictPort: true,
  },
});
