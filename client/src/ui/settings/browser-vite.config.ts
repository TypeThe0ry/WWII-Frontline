import { defineConfig } from 'vite';

// The focused static fixture does not need asset sync, network proxy, or file watching.
export default defineConfig({ publicDir: false, server: { watch: null } });
