/** Scan a published-assets directory for standalone media files (D-PAK: only `.fpak` may be shipped). */
import { readdirSync } from 'node:fs';
import { join, relative } from 'node:path';

export const STANDALONE_EXT = ['.glb', '.gltf', '.ktx2', '.ogg', '.opus', '.wav', '.png', '.jpg', '.jpeg'] as const;

export function findStandalone(dir: string): string[] {
  const out: string[] = [];
  const walk = (d: string) => {
    for (const e of readdirSync(d, { withFileTypes: true })) {
      const p = join(d, e.name);
      if (e.isDirectory()) walk(p);
      else if ((STANDALONE_EXT as readonly string[]).includes(p.slice(p.lastIndexOf('.')).toLowerCase())) out.push(relative(dir, p));
    }
  };
  walk(dir);
  return out.sort();
}

export function assertNoStandalone(dir: string): void {
  const bad = findStandalone(dir);
  if (bad.length) throw new Error(`standalone asset files found under ${dir}: ${bad.join(', ')}`);
}
