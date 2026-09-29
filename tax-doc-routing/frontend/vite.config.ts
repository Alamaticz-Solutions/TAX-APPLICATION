import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';

const frontendRoot = fileURLToPath(new URL('./', import.meta.url));
// Where the dev server proxies the generated GraphQL routes. The default matches the backend's
// default API_PORT; override with VITE_BACKEND_URL when running next to another product.
const backend = process.env.VITE_BACKEND_URL ?? 'http://127.0.0.1:8080';

export default defineConfig({
  plugins: [react()],
  build: {
    target: 'esnext',
    outDir: '../backend/product_dist',
    emptyOutDir: true
  },
  // esbuild 0.28 errors (rather than silently lowering) on modern syntax in pre-bundled deps
  // unless the dep-optimizer target matches the build target.
  optimizeDeps: {
    esbuildOptions: { target: 'esnext' }
  },
  // The design system is imported directly as `@appfw/pds-health-components` (resolved from
  // node_modules, the vendored tarball in vendor/). No product-local alias sits in front of it.
  resolve: {
    dedupe: ['react', 'react-dom']
  },
  server: {
    fs: {
      allow: [frontendRoot]
    },
    port: 5173,
    proxy: {
      '/admin': backend,
      '/system': backend,
      '/tax-routing': backend
    }
  }
});
