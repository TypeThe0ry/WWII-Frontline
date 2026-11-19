import { defineConfig } from '@playwright/test';

const PORT = Number(process.env.PAUSE_SETTINGS_TEST_PORT ?? 5216);
export default defineConfig({
  testDir: '../..',
  testMatch: ['**/settings/settings.browser.spec.ts', '**/audio/engine.browser.spec.ts'],
  timeout: 15_000,
  outputDir: `../../../test-results/settings-audio-${PORT}`,
  workers: 1,
  use: {
    baseURL: `http://localhost:${PORT}`,
    launchOptions: { executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH },
  },
  webServer: {
    command: `pnpm exec vite --config src/ui/settings/browser-vite.config.ts --port ${PORT} --strictPort`,
    cwd: new URL('../../../', import.meta.url).pathname,
    url: `http://localhost:${PORT}/src/ui/settings/browser-fixture.html`,
    reuseExistingServer: false,
  },
});
