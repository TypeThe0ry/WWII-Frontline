import { defineConfig } from 'vitest/config';

// Dev-only: `/game-ws` is proxied to the game server's WebSocket port so a remote preview that forwards only the
// page port can still play (`?server=same`). The server accepts any WebSocket path, so no rewrite is needed.
const wsTarget = process.env.FRONTLINE_WS_TARGET ?? 'ws://127.0.0.1:7000';

export default defineConfig({
  build: { outDir: 'dist', target: 'es2022' },
  server: {
    proxy: {
      '/game-ws': { target: wsTarget, ws: true, changeOrigin: true },
    },
  },
  // Playwright specs live in e2e/ and run via `pnpm test:e2e`, not vitest.
  test: { include: ['src/**/*.test.ts', 'e2e/interop/*.test.ts'] },
});
