import { defineConfig } from 'vitest/config';

export default defineConfig({
  build: { outDir: 'dist', target: 'es2022' },
  // Playwright specs live in e2e/ and run via `pnpm test:e2e`, not vitest.
  test: { include: ['src/**/*.test.ts'] },
});
