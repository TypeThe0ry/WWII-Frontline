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
    files: ['scripts/**/*.mjs', 'src/**/*.mjs'],
    rules: { '@typescript-eslint/no-unused-expressions': 'off' },
    languageOptions: { globals: { console: 'readonly', process: 'readonly', URL: 'readonly' } },
  },
  {
    // ARCHITECTURE §4.3.9: `ui` and `audio` read `game` state and the event bus only — never `render` internals or `net`.
    files: ['src/ui/**/*.{ts,tsx}', 'src/audio/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-imports': ['error', { patterns: [{ group: ['**/render', '**/render/*', '**/net', '**/net/*'], message: 'ui/audio must not import render or net (ARCHITECTURE §4.3.9)' }] }],
    },
  },
  {
    files: ['**/*.js'],
    languageOptions: { globals: { console: 'readonly' } },
  },
);
