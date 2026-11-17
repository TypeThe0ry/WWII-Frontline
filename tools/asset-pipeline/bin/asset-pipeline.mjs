#!/usr/bin/env node
// CLI launcher. Runs the compiled build (dist/, `pnpm -C tools/asset-pipeline build`) if present; otherwise the TypeScript
// sources directly through Node's type stripping (Node >= 22.18 or --experimental-strip-types on 22.12+).
import { existsSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';

const dist = fileURLToPath(new URL('../dist/cli.js', import.meta.url));
const src = fileURLToPath(new URL('../src/cli.ts', import.meta.url));
const target = existsSync(dist) ? dist : src;
const { main } = await import(pathToFileURL(target).href);
process.exitCode = await main(process.argv.slice(2));
