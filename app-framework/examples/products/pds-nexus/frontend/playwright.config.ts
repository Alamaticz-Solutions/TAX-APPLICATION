/**
 * Playwright configuration for the Nexus IX real-journey proof.
 *
 * EXECUTION: DEFERRED_TO_B_HOST. The journey spec drives the real product
 * frontend against the real backend binary with the chat-gated IX runtime
 * mounted and a real PostgreSQL behind it; that composition is the B_HOST
 * leaf's recorded responsibility. This configuration intentionally has no
 * webServer block and no mocking: the B_HOST check script provides
 * IX_JOURNEY_BASE_URL for the served frontend and the spec fails closed
 * without it.
 */

import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 120_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  retries: 0,
  reporter: [['list'], ['json', { outputFile: 'test-results/ix-real-journey-report.json' }]],
  use: {
    baseURL: process.env.IX_JOURNEY_BASE_URL,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure'
  }
});
