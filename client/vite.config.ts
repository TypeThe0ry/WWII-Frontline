import { defineConfig } from 'vitest/config';

// Dev-only: `/game-ws` is proxied to the game server's WebSocket port so a remote preview that forwards only the
// page port can still play (`?server=same`). The server accepts any WebSocket path, so no rewrite is needed.
const wsTarget = process.env.FRONTLINE_WS_TARGET ?? 'ws://127.0.0.1:7000';

export default defineConfig({
  // Keep Preact's renderer, hooks and signals on the same optimized instance across lazy routes.
  optimizeDeps: { include: ['preact', 'preact/hooks', 'preact/compat', 'preact/jsx-runtime', 'preact/jsx-dev-runtime', '@preact/signals'] },
  build: { outDir: 'dist', target: 'es2022' },
  server: {
    proxy: {
      '/v1': { target: process.env.FRONTLINE_GATEWAY_TARGET ?? 'http://127.0.0.1:8080', changeOrigin: true },
      '/game-ws': { target: wsTarget, ws: true, changeOrigin: true },
    },
  },
  // Playwright specs live in e2e/ and run via `pnpm test:e2e`, not vitest.
  test: { include: ['src/**/*.test.ts', 'e2e/interop/*.test.ts'] },
});
