/** Exact M1 Medium model inputs, selected from the existing free-source inventory. */
import { copyFileSync, mkdirSync, readFileSync } from 'node:fs';
import { basename, dirname, join, relative } from 'node:path';
import { lookupByRawPath, type Ledger } from './ledger.ts';
import { sha256File } from './ingest.ts';
import { normalize } from './normalize.ts';
import { processMesh } from './process.ts';
import { pack } from './pack.ts';
import { checkSourceIndex } from './source-index.ts';

const MODELS = [
  ['V-NAT-002', 'tree_small_02', 'veg_tree_small_02'],
  ['V-NAT-004', 'fern_02', 'veg_fern_02'],
  ['V-NAT-005', 'rock_05', 'prop_rock_05'],
  ['V-PRP-004', 'ammo_box', 'prop_ammo_box'],
  ['V-PRP-006', 'wooden_crate_02', 'prop_wooden_crate_02'],
] as const;

export function carentanSources(root: string, ledger: Ledger) {
  const index = join(root, 'assets/manifests/incoming-index.json');
  const inventory = checkSourceIndex(index, ledger, root);
  const { files } = JSON.parse(readFileSync(index, 'utf8')) as {
    files: { path: string; sha256: string; size_bytes: number }[];
  };
  return MODELS.map(([req, slug, logicalId]) => {
    const dir = `assets/incoming/${req}/polyhaven-${slug}`;
    const model = `${dir}/${slug}_2k.gltf`;
    const row = lookupByRawPath(ledger, model);
    const inputs = files.filter((file) => file.path.startsWith(`${dir}/`));
    if (row?.req_id !== req || row.status !== 'OK' || inventory.coverage.get(req) !== 'real'
      || !inputs.some((file) => file.path === model)) {
      throw new Error(`${model}: no approved M1 Medium model`);
    }
    return { slug, dir, model, assetId: row.asset_id, logicalId, inputs };
  });
}

export async function packCarentanSources(root: string, incoming: string, work: string, out: string, ledger: Ledger) {
  const models = carentanSources(root, ledger);
  for (const { dir, model, logicalId, inputs, assetId } of models) {
    const raw = join(work, assetId, 'raw');
    for (const { path, sha256, size_bytes } of inputs) {
      const source = join(incoming, relative('assets/incoming', path));
      const dest = join(raw, relative(dir, path));
      mkdirSync(dirname(dest), { recursive: true });
      copyFileSync(source, dest);
      const actual = sha256File(dest);
      if (actual.sha256 !== sha256 || actual.bytes !== size_bytes) throw new Error(`${path}: source differs from incoming-index.json`);
    }
    const gltf = JSON.parse(readFileSync(join(raw, basename(model)), 'utf8')) as {
      images?: { uri?: string }[]; buffers?: { uri?: string }[];
    };
    const approvedPaths = new Set(inputs.map((file) => relative(dir, file.path)));
    for (const { uri } of [...(gltf.images ?? []), ...(gltf.buffers ?? [])]) {
      if (uri && !approvedPaths.has(uri)) throw new Error(`${model}: unregistered external URI ${uri}`);
    }
    await normalize({ work, ledger, assetId, logicalId, input: basename(model), unitScale: 1, up: 'Y', forward: '+Z', bakeTextureTransforms: true });
    await processMesh({ work, ledger, assetId, logicalId, cls: { texture_profile: 'bg' }, noCollision: true });
  }
  await pack({ work, ledger, out, assets: models.map(({ assetId }) => assetId) });
}
