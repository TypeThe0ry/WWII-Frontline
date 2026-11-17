// usage: node tools/asset-pipeline/scripts/assert-no-standalone.ts [dir=assets/processed]
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { assertNoStandalone } from '../src/fpak/standalone.ts';

const dir = resolve(process.argv[2] ?? 'assets/processed');
if (!existsSync(dir)) {
  console.log(`assert-no-standalone: ${dir} does not exist (nothing to check)`);
} else {
  try {
    assertNoStandalone(dir);
    console.log(`assert-no-standalone: ${dir} contains no standalone .glb/.gltf/.ktx2/.ogg/.opus/.wav/.png/.jpg files`);
  } catch (e) {
    console.error(`assert-no-standalone: ${(e as Error).message}`);
    process.exitCode = 1;
  }
}
