import js from '@eslint/js';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  { ignores: ['dist/', 'node_modules/', 'public/'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    // Browser-side test shim injected via page.addInitScript().
    files: ['e2e/**/*.js'],
    languageOptions: { globals: { window: 'readonly', GPUCanvasContext: 'readonly', GPUTextureUsage: 'readonly' } },
  },
  {
    // Node-side helper scripts (scripts/sync-map.mjs).
    files: ['scripts/**/*.mjs'],
    languageOptions: { globals: { console: 'readonly', process: 'readonly' } },
  },
  {
    files: ['**/*.js'],
    languageOptions: { globals: { console: 'readonly' } },
  },
);
