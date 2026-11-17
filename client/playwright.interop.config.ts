import { defineConfig } from '@playwright/test';

// Browser interop matrix (M0-028); starts its own harness page server and frontline-server (see e2e/interop/).
//   pnpm -C client exec playwright test -c playwright.interop.config.ts
export default defineConfig({
  testDir: 'e2e/interop',
  globalSetup: './e2e/interop/global-setup.ts',
  testMatch: 'interop.spec.ts',
  timeout: 90_000,
  workers: 1,
  reporter: [['list']],
});
