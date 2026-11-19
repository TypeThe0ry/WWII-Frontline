import { defineConfig } from '@playwright/test';

const PORT = Number(process.env.FRONTLINE_PLAYWRIGHT_PORT ?? 5199);

// Software-rendered Chromium: WebGL2 via SwiftShader; WebGPU only through the SwiftShader adapter
// (see e2e/webgpu-swiftshader-shim.js for why presenting is stubbed).
export default defineConfig({
  testDir: 'e2e',
  // *.test.ts under e2e/ are vitest files (e2e/interop/fixtures.test.ts), not Playwright specs.
  testIgnore: ['**/*.test.ts'],
  timeout: 60_000,
  workers: 1,
  reporter: [['list']],
  use: {
    baseURL: `http://localhost:${PORT}`,
    viewport: { width: 960, height: 540 },
    launchOptions: {
      args: [
        '--enable-unsafe-webgpu',
        '--enable-unsafe-swiftshader',
        '--enable-features=Vulkan',
        '--use-angle=swiftshader',
        '--use-webgpu-adapter=swiftshader',
        '--ignore-gpu-blocklist',
      ],
    },
  },
  webServer: {
    command: `node scripts/sync-map.mjs && pnpm exec vite --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}/`,
    reuseExistingServer: false,
    timeout: 60_000,
  },
});
