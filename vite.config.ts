import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { readFileSync } from 'node:fs';

const packageJsonPath = new URL('./package.json', import.meta.url);
const { version } = JSON.parse(readFileSync(packageJsonPath, 'utf-8')) as { version: string };

export default defineConfig({
  plugins: [react()],

  clearScreen: false,
  define: {
    __APP_VERSION__: JSON.stringify(version),
  },

  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
});
