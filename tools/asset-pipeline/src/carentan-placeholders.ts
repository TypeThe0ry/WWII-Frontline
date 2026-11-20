/** Public procedural proxies for the Carentan medium bundle. Inputs are checked-in gameplay data and primitive recipes. */
import { randomUUID } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { SAMPLE_RATE } from './audio/encode.ts';
import { packAudio, type AudioItem } from './audio/pack-audio.ts';
import { writeWav } from './audio/wav.ts';
import { encodeMeshEntry, type MeshEntry } from './fpak/entry-mesh.ts';
import { Codec } from './fpak/format.ts';
import { FpakWriter, publishPath, type FpakBuild } from './fpak/writer.ts';
import type { Overrides } from './manifest.ts';

type Vec3 = [number, number, number];
type Point = [number, number];
type Face = Vec3[];

/** Extrude a convex, counter-clockwise XY outline along Z. */
function prism(outline: Point[], z: number, depth: number): Face[] {
  const front = outline.map<Vec3>(([x, y]) => [x, y, z + depth / 2]);
  const back = outline.map<Vec3>(([x, y]) => [x, y, z - depth / 2]);
  return [front, [...back].reverse(), ...outline.map((_, i) => {
    const next = (i + 1) % outline.length;
    return [back[i]!, back[next]!, front[next]!, front[i]!];
  })];
}

function box([w, h, d]: Vec3, [x, y, z]: Vec3): Face[] {
  return prism([[x - w / 2, y - h / 2], [x + w / 2, y - h / 2], [x + w / 2, y + h / 2], [x - w / 2, y + h / 2]], z, d);
}

function barrel(length: number, diameter: number, [x, y, z]: Vec3): Face[] {
  return prism(Array.from({ length: 8 }, (_, i): Point => {
    const a = i * Math.PI / 4;
    return [x + Math.cos(a) * diameter / 2, y + Math.sin(a) * diameter / 2];
  }), z, length);
}

function solidMesh(faces: Face[], rgb: Vec3): MeshEntry {
  const min: Vec3 = [Infinity, Infinity, Infinity];
  const max: Vec3 = [-Infinity, -Infinity, -Infinity];
  const positions: number[] = [];
  const normals: number[] = [];
  const indices: number[] = [];
  for (const face of faces) {
    const [a, b, c] = face as [Vec3, Vec3, Vec3, ...Vec3[]];
    const [ux, uy, uz] = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    const [vx, vy, vz] = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    const normal = [uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx];
    const length = Math.hypot(...normal);
    const first = positions.length / 3;
    for (const p of face) {
      positions.push(...p);
      normals.push(...normal.map((n) => n / length));
      for (const axis of [0, 1, 2] as const) {
        min[axis] = Math.min(min[axis], p[axis]);
        max[axis] = Math.max(max[axis], p[axis]);
      }
    }
    for (let i = 1; i < face.length - 1; i++) indices.push(first, first + i, first + i + 1);
  }
  const attr = (semantic: string, values: number[]) => ({
    semantic, componentType: 5126, elements: 3, normalized: false, data: new Uint8Array(Float32Array.from(values).buffer),
  });
  return {
    bbox: { min, max },
    materials: [{ baseColor: [...rgb, 1], metallic: 0, roughness: 1, emissive: [0, 0, 0], textures: ['', '', '', ''] }],
    primitives: [{
      materialIndex: 0, matrix: [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
      vertexCount: positions.length / 3, indexSize: 2, indices: Uint32Array.from(indices),
      attributes: [attr('POSITION', positions), attr('NORMAL', normals)],
    }],
  };
}

interface RifleDimensions {
  requirement_id: string;
  length_m: number;
  barrel_length_m: number;
  barrel_diameter_m: number;
  receiver_height_m: number;
  receiver_width_m: number;
  magazine_length_m: number;
  magazine_height_m: number;
  magazine_width_m: number;
}

function rifle(d: RifleDimensions): Face[] {
  // Match the client greybox's grip origin and +Z muzzle (placeholder-weapon.ts).
  const rear = -0.45 * d.length_m;
  const receiverLength = d.length_m - d.barrel_length_m;
  return [
    ...box([d.receiver_width_m, d.receiver_height_m, receiverLength], [0, 0, rear + receiverLength / 2]),
    ...barrel(d.barrel_length_m, d.barrel_diameter_m, [0, d.receiver_height_m / 4, rear + receiverLength + d.barrel_length_m / 2]),
    ...box([d.magazine_width_m, d.magazine_height_m, d.magazine_length_m], [0, -(d.receiver_height_m + d.magazine_height_m) / 2, 0]),
  ];
}

function soldier(height: number, radius: number): Face[] {
  return [
    ...box([1.3 * radius, 0.42 * height, radius], [0, 0.62 * height, 0]),
    ...box([1.9 * radius, 0.08 * height, radius], [0, 0.75 * height, 0]),
    ...box([radius, 0.18 * height, radius], [0, 0.91 * height, 0]),
    ...[-1, 1].flatMap((side) => box([0.47 * radius, 0.42 * height, radius], [side * 0.31 * radius, 0.21 * height, 0])),
  ];
}

function building(w: number, h: number, d: number, roof: number): Face[] {
  const t = 0.25;
  const door = 1.5;
  const top = 2.2;
  const front = d / 2 - t / 2;
  return [
    ...box([t, h, d], [-(w - t) / 2, h / 2, 0]),
    ...box([t, h, d], [(w - t) / 2, h / 2, 0]),
    ...box([w - 2 * t, h, t], [0, h / 2, -front]),
    ...[-1, 1].flatMap((side) => box([(w - door) / 2 - t, h, t], [side * (w + door - 2 * t) / 4, h / 2, front])),
    ...box([door, h - top, t], [0, (h + top) / 2, front]),
    ...prism([[-w / 2, h], [w / 2, h], [0, h + roof]], 0, d),
  ];
}

function tank(w: number, h: number, d: number): Face[] {
  return [
    ...[-1, 1].flatMap((side) => box([w * 0.22, h * 0.28, d], [side * w * 0.39, h * 0.14, 0])),
    ...box([w * 0.81, h * 0.39, d * 0.92], [0, h * 0.405, 0]),
    ...box([w * 0.62, h * 0.4, d * 0.43], [0, h * 0.8, 0]),
    ...barrel(d * 0.27, w * 0.055, [0, h * 0.79, d * 0.35]),
  ];
}

function publicData<T>(path: string): T {
  return JSON.parse(readFileSync(new URL(`../../../data/${path}`, import.meta.url), 'utf8')) as T;
}

async function buildVisuals(mark: (id: string, reqId: string) => void): Promise<FpakBuild> {
  const writer = new FpakWriter();
  const grey: Vec3 = [0.27, 0.28, 0.28];
  const olive: Vec3 = [0.28, 0.31, 0.18];
  const add = async (id: string, reqId: string, faces: Face[], rgb: Vec3 = grey, handHeld = false) => {
    const payload = await encodeMeshEntry(solidMesh(faces, rgb));
    for (const suffix of handHeld ? ['_fp', '_tp'] : ['']) writer.add(`${id}@lod0${suffix}`, 'mesh', Codec.meshV1, payload);
    mark(id, reqId);
  };

  const { weapons } = publicData<{ weapons: Record<string, RifleDimensions> }>('placeholders/weapon_models.json');
  for (const id of Object.keys(weapons).sort()) {
    const dims = weapons[id]!;
    await add(`wpn_ph_${id}`, dims.requirement_id, rifle(dims), grey, true);
  }
  // Standalone block silhouettes for M1 requirements also associated with internal-only ledger rows.
  await add('wpn_ph_ger_mp40', 'V-WPN-GE-004', [
    ...box([0.055, 0.085, 0.55], [0, 0, -0.09]), ...box([0.025, 0.025, 0.22], [0, 0.02, 0.295]), ...box([0.032, 0.21, 0.045], [0, -0.145, 0.09]),
  ], grey, true);
  await add('wpn_ph_ger_stg44', 'V-WPN-GE-005', [
    ...box([0.07, 0.105, 0.69], [0, 0, -0.10]), ...box([0.027, 0.027, 0.25], [0, 0.02, 0.37]), ...box([0.032, 0.18, 0.078], [0, -0.14, 0.035]),
  ], grey, true);
  await add('wpn_ph_ger_p38', 'V-WPN-GE-008', [
    ...box([0.036, 0.045, 0.14], [0, 0.032, 0.02]), ...box([0.019, 0.019, 0.074], [0, 0.045, 0.127]), ...box([0.033, 0.10, 0.05], [0, -0.035, -0.018]),
  ], grey, true);

  const { capsule } = publicData<{ capsule: { height_stand_m: number; radius_m: number } }>('roles/rifleman.json');
  const infantry = soldier(capsule.height_stand_m, capsule.radius_m);
  await add('chr_ph_us_infantry', 'V-CHR-US-001', infantry, olive);
  await add('chr_ph_ger_infantry', 'V-CHR-GE-001', infantry);
  const { historical } = publicData<{ historical: { dimensions_m: { width: number; height: number; length: number } } }>('vehicles/us_m4a1_75.json');
  const { width, height, length } = historical.dimensions_m;
  await add('veh_ph_us_m4a1_75', 'V-VEH-US-003', tank(width, height, length), olive);

  // Representative standalone silhouettes. The map still supplies instance geometry and collision.
  await add('bld_ph_stone_house', 'V-BLD-001', building(8, 5.4, 6, 2));
  await add('bld_ph_farm_barn', 'V-BLD-002', building(11, 4.2, 7, 2.6));
  await add('bld_ph_village_church', 'V-BLD-003', [...building(9, 6, 14, 3), ...box([3.2, 10, 3], [0, 5, -5.5])]);
  await add('veg_ph_bocage_segment', 'V-NAT-001', [
    ...prism([[-0.5, 0], [0.5, 0], [0.38, 0.66], [-0.38, 0.66]], 0, 6), ...box([0.85, 1.35, 6], [0, 1.325, 0]),
  ], olive);
  await add('veg_ph_deciduous_tree', 'V-NAT-002', [...box([0.35, 4.2, 0.35], [0, 2.1, 0]), ...box([3.4, 3.9, 3.2], [0, 5.25, 0])], olive);
  await add('prop_ph_sandbag_wall', 'V-PRP-001', Array.from({ length: 3 }, (_, row) => (
    Array.from({ length: 4 }, (_, col) => box([0.54, 0.22, 0.43], [(col - 1.5) * 0.55 + (row % 2) * 0.12, 0.11 + row * 0.22, 0])).flat()
  )).flat());
  await add('prop_ph_supply_crate', 'V-PRP-004', [
    ...box([0.84, 0.44, 0.48], [0, 0.22, 0]), ...box([0.92, 0.08, 0.54], [0, 0.48, 0]),
  ], olive);
  return writer.finish();
}

interface Cue {
  name: string;
  req: string;
  hz: number;
  ms: number;
  cls: AudioItem['cls'];
}

function cues(): Cue[] {
  const rifles = [
    ['us_m1_garand', 'A-WPN-001', 330], ['ger_kar98k', 'A-WPN-009', 370], ['ger_mp40', 'A-WPN-011', 415],
  ] as const;
  const layers = [['near', 110, 1], ['mid', 155, 0.75], ['far', 210, 0.5]] as const;
  return [
    ...rifles.flatMap(([id, req, hz]) => layers.map(([layer, ms, ratio]): Cue => ({
      name: `wpn_${id}_fire_${layer}`, req, hz: hz * ratio, ms, cls: `gun_${layer}`,
    }))),
    { name: 'wpn_us_m1_garand_ping', req: 'A-WPN-001', hz: 880, ms: 105, cls: 'foley' },
    { name: 'wpn_ger_kar98k_bolt', req: 'A-WPN-009', hz: 530, ms: 90, cls: 'foley' },
    { name: 'wpn_reload', req: 'A-WPN-017', hz: 580, ms: 115, cls: 'foley' },
    { name: 'grenade_blast', req: 'B-EXP-001', hz: 120, ms: 260, cls: 'gun_mid' },
    { name: 'bullet_whizz', req: 'C-BUL-001', hz: 690, ms: 140, cls: 'foley' },
    { name: 'bullet_impact_dirt', req: 'C-BUL-002', hz: 260, ms: 100, cls: 'foley' },
    { name: 'bullet_impact_metal', req: 'C-BUL-002', hz: 740, ms: 120, cls: 'foley' },
    { name: 'footstep_grass', req: 'E-FTS-001', hz: 210, ms: 130, cls: 'foley' },
    { name: 'footstep_mud', req: 'E-FTS-001', hz: 165, ms: 150, cls: 'foley' },
    { name: 'footstep_wood', req: 'E-FTS-001', hz: 290, ms: 120, cls: 'foley' },
    { name: 'gear_rustle', req: 'E-GER-001', hz: 240, ms: 175, cls: 'foley' },
    { name: 'ui_confirm', req: 'H-UI-001', hz: 660, ms: 110, cls: 'ui' },
    { name: 'ui_capture', req: 'H-UI-002', hz: 790, ms: 190, cls: 'ui' },
  ];
}

function cueWav(cue: Cue): Uint8Array {
  // Abstract encoder/test cues. Stable PCM made from a swept tone with a smooth onset and release.
  const count = Math.round(cue.ms * SAMPLE_RATE / 1000);
  const samples = Float32Array.from({ length: count }, (_, i) => {
    const u = i / (count - 1);
    const phase = 2 * Math.PI * cue.hz * (i / SAMPLE_RATE) * (1 - 0.14 * u);
    return 0.18 * Math.sin(phase) * Math.sin(Math.PI * u) ** 2;
  });
  return writeWav({ sampleRate: SAMPLE_RATE, samples });
}

async function buildSounds(mark: (id: string, reqId: string) => void): Promise<FpakBuild> {
  const temp = mkdtempSync(join(tmpdir(), 'fl-carentan-ph-'));
  try {
    const items = cues().map((cue): AudioItem => {
      const id = `snd_ph_${cue.name}`;
      const src = join(temp, `${id}.wav`);
      writeFileSync(src, cueWav(cue));
      mark(id, cue.req);
      return { logical_id: id, src, cls: cue.cls };
    });
    return (await packAudio(items)).build;
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
}

function publish(out: string, build: FpakBuild): void {
  const dest = join(out, publishPath(build.sha256).slice(1));
  mkdirSync(dirname(dest), { recursive: true });
  const temp = `${dest}.${randomUUID()}.tmp`;
  try {
    writeFileSync(temp, build.bytes, { flag: 'wx' });
    renameSync(temp, dest);
  } finally {
    rmSync(temp, { force: true });
  }
}

export async function buildCarentanPlaceholders(out: string): Promise<Overrides> {
  const overrides: Overrides = {};
  const mark = (id: string, req_id: string) => { overrides[id] = { placeholder: true, req_id, tier: ['medium'] }; };
  const visuals = await buildVisuals(mark);
  const sounds = await buildSounds(mark);
  publish(out, visuals);
  publish(out, sounds);
  return overrides;
}
