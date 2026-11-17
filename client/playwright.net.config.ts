import { defineConfig } from '@playwright/test';

// Net-only e2e (M0-021): no dev-server dependency (the spec starts its own harness server and frontline-server),
// so it can run next to other Playwright runs that occupy the main config's port.
//   pnpm -C client exec playwright test -c playwright.net.config.ts
export default defineConfig({
  testDir: 'e2e',
  testMatch: 'net.spec.ts',
  timeout: 90_000,
  workers: 1,
  reporter: [['list']],
});
