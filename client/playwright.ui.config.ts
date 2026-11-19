import { defineConfig } from '@playwright/test';
import base from './playwright.config';

const PORT = Number(process.env.FRONTLINE_UI_PORT ?? 5217);
export default defineConfig({
  ...base,
  testMatch: ['ui-lobby.spec.ts', 'ui-match.spec.ts'],
  use: {
    ...base.use,
    baseURL: `http://127.0.0.1:${PORT}`,
    launchOptions: { ...base.use?.launchOptions, executablePath: process.env.FRONTLINE_CHROME },
  },
  webServer: {
    command: `node scripts/sync-map.mjs && pnpm exec vite --host 127.0.0.1 --port ${PORT} --strictPort`,
    url: `http://127.0.0.1:${PORT}`,
    reuseExistingServer: false,
    timeout: 60_000,
  },
});
