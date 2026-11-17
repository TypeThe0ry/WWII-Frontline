import { defineConfig } from '@playwright/test';

// Networked-game e2e (M0-022): the real app in headless Chromium (software WebGL2) against a real frontline-server.
// The spec starts its own Vite server and server process (no webServer here), so it can run next to other Playwright runs.
//   pnpm -C client exec playwright test -c playwright.game.config.ts
export default defineConfig({
  testDir: 'e2e',
  testMatch: ['game-net.spec.ts', 'actors-net.spec.ts', 'low-fps.spec.ts'],
  timeout: 90_000,
  workers: 1,
  reporter: [['list']],
  use: {
    viewport: { width: 640, height: 360 },
    launchOptions: { args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] },
  },
});
