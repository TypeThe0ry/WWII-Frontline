import { defineConfig } from '@playwright/test';

// The spec owns its gateway, sim server and Vite instance. No graphics or pointer lock needed.
export default defineConfig({
  testDir: 'e2e',
  testMatch: 'garrison-loop.spec.ts',
  // Stock generation, walking the legal HQ setback with a crate, building and respawn are real-time.
  timeout: 210_000,
  workers: 1,
  reporter: [['list']],
  use: {
    headless: true,
    launchOptions: {
      executablePath: process.env.FRONTLINE_CHROME,
      args: ['--disable-gpu', '--disable-background-timer-throttling', '--disable-renderer-backgrounding',
        ...(process.env.FRONTLINE_M1_DEMO_DIR ? ['--disable-features=Vulkan', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] : [])],
    },
  },
});
