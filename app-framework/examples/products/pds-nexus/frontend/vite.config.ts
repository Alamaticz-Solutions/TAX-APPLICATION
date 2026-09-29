import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';

const frontendRoot = fileURLToPath(new URL('./', import.meta.url));

// Consume rule (assignment-0002 / amendment A3): the PDS packages resolve
// exclusively through node_modules from the Integration-receipted archives.
// No adjacent-repository source alias and no absolute path may appear here.
export default defineConfig({
  plugins: [react()],
  build: {
    target: 'esnext',
    outDir: '../backend/product_dist',
    emptyOutDir: true
  },
  resolve: {
    alias: {
      'react/jsx-runtime': fileURLToPath(new URL('./node_modules/react/jsx-runtime.js', import.meta.url)),
      'react-dom': fileURLToPath(new URL('./node_modules/react-dom', import.meta.url)),
      'react': fileURLToPath(new URL('./node_modules/react', import.meta.url))
    },
    dedupe: ['react', 'react-dom']
  },
  server: {
    fs: {
      allow: [frontendRoot]
    },
    port: 5173,
    proxy: {
      '/admin': 'http://127.0.0.1:8080',
      '/system': 'http://127.0.0.1:8080',
      '/denovo-workflow': 'http://127.0.0.1:8080',
      '/chat': 'http://127.0.0.1:8080'
    }
  }
});
