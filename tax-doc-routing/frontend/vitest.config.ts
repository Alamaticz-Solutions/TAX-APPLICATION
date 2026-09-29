import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

// Frontend test backbone. Unit + component tests (this config) run under
// jsdom. Real-browser E2E / accessibility coverage (Playwright + axe-core,
// against a real running Vite server, not jsdom) lives in tests/e2e/ — run
// with `npm run test:e2e`, configured in playwright.config.ts. The two never
// overlap: this config's `include` only ever matches files under src/.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{ts,tsx}']
  }
});
