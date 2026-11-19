/**
 * Placeholder naming (ASSET_GAPS 3.1): materials `PH_{requirementId}`, nodes `PH_{requirementId}__{instance}`.
 * Every grey-box object is bound to the requirement id of the real asset that will replace it; the registry
 * feeds `window.__placeholders` (the "missing asset report" of M1-001).
 */
import { Color3 } from '@babylonjs/core/Maths/math.color';
import { StandardMaterial } from '@babylonjs/core/Materials/standardMaterial';
import type { Scene } from '@babylonjs/core/scene';

export const REQ = {
  CHR_US: 'V-CHR-US-001',
  CHR_GE: 'V-CHR-GE-001',
  WPN_US: 'V-WPN-US-001',
  WPN_GE: 'V-WPN-GE-001',
  FX_FLASH: 'V-FX-001',
  FX_TRACER: 'V-FX-006', // tracer line, ground dust and impact marks all stand in for V-FX-006
} as const;

/** Every placeholder mesh / material name has to match this (unit test walks the scene). */
export const PH_NAME_RE = /^PH_V-[A-Z]+-[A-Z0-9-]+(__.+)?$/;

export const SOCKET = {
  muzzle: 'socket_muzzle',
  ejection: 'socket_ejection',
  magazine: 'socket_magazine',
  handR: 'socket_hand_r',
  handL: 'socket_hand_l',
  head: 'socket_head',
  back: 'socket_back',
} as const;

export type Faction = 'us' | 'ger';

/** Debug tint per faction (not final art): US olive green, GER grey. */
export const FACTION_COLOR: Readonly<Record<Faction, string>> = { us: '#6B7B3A', ger: '#7A7F85' };

export const characterReq = (f: Faction): string => (f === 'us' ? REQ.CHR_US : REQ.CHR_GE);
export const weaponReq = (f: Faction): string => (f === 'us' ? REQ.WPN_US : REQ.WPN_GE);

export function phName(req: string, instance?: string | number): string {
  return instance === undefined ? `PH_${req}` : `PH_${req}__${instance}`;
}

export interface PlaceholderEntry {
  id: string;
  instances: number;
  placeholder: true;
}

export class PlaceholderRegistry {
  private readonly counts = new Map<string, number>();

  /** Declares a requirement id (0 live instances) so the report lists it as soon as its sources exist. */
  declare(req: string): void {
    if (!this.counts.has(req)) this.counts.set(req, 0);
  }

  add(req: string, n = 1): void {
    this.counts.set(req, Math.max(0, (this.counts.get(req) ?? 0) + n));
  }

  report(): PlaceholderEntry[] {
    return [...this.counts.entries()].map(([id, instances]) => ({ id, instances, placeholder: true as const })).sort((a, b) => (a.id < b.id ? -1 : 1));
  }

  /** `window.__placeholders` (live getter). */
  install(target: object): void {
    Object.defineProperty(target, '__placeholders', { configurable: true, get: () => this.report() });
  }
}

const materials = new WeakMap<Scene, Map<string, StandardMaterial>>();

/** Flat-colour material `PH_{req}` (one per requirement and scene; `tag` distinguishes variants of one requirement). */
export function placeholderMaterial(scene: Scene, req: string, hex: string, opts: { emissive?: boolean; tag?: string } = {}): StandardMaterial {
  let per = materials.get(scene);
  if (!per) {
    per = new Map();
    materials.set(scene, per);
  }
  const name = opts.tag ? `PH_${req}__${opts.tag}` : `PH_${req}`;
  const hit = per.get(name);
  if (hit) return hit;
  const m = new StandardMaterial(name, scene);
  const c = Color3.FromHexString(hex);
  m.specularColor = Color3.Black();
  if (opts.emissive) {
    m.emissiveColor = c;
    m.disableLighting = true;
  } else {
    m.diffuseColor = c;
    m.ambientColor = c.scale(0.6);
  }
  per.set(name, m);
  return m;
}
