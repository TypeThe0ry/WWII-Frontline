import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, it } from 'vitest';
import { verifyCarentanMedium } from '../src/carentan-medium.ts';
import { buildCarentanPlaceholders } from '../src/carentan-placeholders.ts';
import { BufferSource, Codec, FpakReader, assertOpaque, decodeAudioEntry, decodeMeshEntry, publishPath } from '../src/fpak/index.ts';
import { buildManifest } from '../src/manifest.ts';

it('publishes repeatable public mesh and audio proxies with catalogued placeholder requirements', async () => {
  const temp = mkdtempSync(join(tmpdir(), 'fl-test-carentan-ph-'));
  try {
    const out = join(temp, 'packs');
    const overrides = await buildCarentanPlaceholders(out);
    const files = (dir: string) => readdirSync(dir, { recursive: true, withFileTypes: true })
      .filter((entry) => entry.isFile()).map((entry) => join(entry.parentPath, entry.name).slice(dir.length)).sort();
    const paths = files(out);
    expect(paths).toHaveLength(2);
    const repeat = join(temp, 'repeat');
    expect(await buildCarentanPlaceholders(repeat)).toEqual(overrides);
    expect(files(repeat)).toEqual(paths);

    const seen = new Map<string, Set<string>>();
    for (const path of paths) {
      const bytes = readFileSync(out + path);
      expect(readFileSync(repeat + path).equals(bytes)).toBe(true);
      expect(path).toBe(publishPath(createHash('sha256').update(bytes).digest('hex')));
      const reader = await FpakReader.open(new BufferSource(bytes));
      expect(new Set(reader.entries().map((entry) => entry.kind)).size).toBe(1);
      for (const entry of reader.entries()) {
        const [id, variant] = entry.logical_id.split('@') as [string, string];
        const variants = seen.get(id) ?? new Set<string>();
        expect(variants.has(variant)).toBe(false);
        variants.add(variant);
        seen.set(id, variants);
        const payload = await reader.read(entry.logical_id);
        if (entry.kind === 'mesh') {
          expect(entry.codec).toBe(Codec.meshV1);
          const mesh = await decodeMeshEntry(payload);
          for (const axis of [0, 1, 2] as const) expect(mesh.bbox.max[axis]).toBeGreaterThan(mesh.bbox.min[axis]);
          if (id === 'wpn_ph_us_m1_garand' && variant === 'lod0_fp') {
            await assertOpaque(payload);
            const data = JSON.parse(readFileSync(new URL('../../../data/placeholders/weapon_models.json', import.meta.url), 'utf8'));
            const expected = data.weapons.us_m1_garand.length_m as number;
            expect(mesh.bbox.min[2]).toBeCloseTo(-0.45 * expected, 5);
            expect(mesh.bbox.max[2]).toBeCloseTo(0.55 * expected, 5);
          }
        } else {
          expect(entry.kind).toBe('audio');
          expect(entry.codec).toBe(Codec.audio);
          const audio = decodeAudioEntry(payload);
          expect(audio).toMatchObject({ container: variant === 'opus' ? 'webm-opus' : 'mp4-aac', channels: 1, sampleRate: 48000, preload: true, loop: false });
          expect(audio.payload.length).toBeGreaterThan(0);
        }
      }
    }
    expect([...seen.keys()].sort()).toEqual(Object.keys(overrides).sort());
    expect([...seen.get('wpn_ph_us_m1_garand')!].sort()).toEqual(['lod0_fp', 'lod0_tp']);
    for (const [id, variants] of seen) {
      if (id.startsWith('snd_ph_')) expect([...variants].sort()).toEqual(['aac', 'opus']);
    }

    const catalog = JSON.parse(readFileSync(new URL('../../../assets/requirements.json', import.meta.url), 'utf8')) as {
      requirements: { id: string; milestone: string; kind: string }[];
    };
    const requirements = new Map(catalog.requirements.filter(({ milestone }) => ['M0', 'M1'].includes(milestone)).map((req) => [req.id, req.kind]));
    const ledger = new URL('../../../LICENSES.md', import.meta.url).pathname;
    const manifest = await buildManifest({ packs: out, ledger, work: join(temp, 'work'), bundles: { map_carentan_medium: ['*'] }, budgets: {}, overrides });
    for (const [id, entry] of Object.entries(manifest.assets)) {
      expect(entry).toMatchObject(overrides[id]!);
      expect(entry.license_ref).toBeUndefined();
      expect(requirements.get(entry.req_id!)).toBe(id.startsWith('snd_ph_') ? 'audio' : 'visual');
    }
    expect(manifest.assets.chr_ph_ger_infantry?.req_id).toBe('V-CHR-GE-001');
    expect(manifest.assets.wpn_ph_ger_mp40?.req_id).toBe('V-WPN-GE-004');
    expect(manifest.assets.snd_ph_bullet_impact_metal?.req_id).toBe('C-BUL-002');
    const physicalSize = paths.reduce((bytes, path) => bytes + readFileSync(out + path).length, 0);
    const entryCount = [...seen.values()].reduce((count, variants) => count + variants.size, 0);
    expect(await verifyCarentanMedium(manifest, out)).toEqual({ packs: 2, entries: entryCount, bytes: physicalSize });
    const damagedPath = out + paths[0];
    const damaged = readFileSync(damagedPath);
    damaged[12] = 1; // Reserved header byte, outside payload and index checksums.
    writeFileSync(damagedPath, damaged);
    await expect(verifyCarentanMedium(manifest, out)).rejects.toThrow('round-trip did not reproduce');
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}, 60_000);
