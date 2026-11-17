import js from '@eslint/js';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  { ignores: ['dist/', 'node_modules/'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    // Browser-side test shim injected via page.addInitScript().
    files: ['e2e/**/*.js'],
    languageOptions: { globals: { window: 'readonly', GPUCanvasContext: 'readonly', GPUTextureUsage: 'readonly' } },
  },
  {
    files: ['**/*.js'],
    languageOptions: { globals: { console: 'readonly' } },
  },
);
