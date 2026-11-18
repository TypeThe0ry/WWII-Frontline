/** Marker placement rules (GAMEDESIGN §16; local prediction only, the server is authoritative). */
export type MarkerKind = 'attention' | 'enemyInfantry' | 'enemyVehicle' | 'needMedic' | 'needAmmo' | 'orderAttack' | 'orderDefend' | 'orderMove';

export const MARKER_TTL_S: Readonly<Record<MarkerKind, number>> = {
  attention: 15,
  enemyInfantry: 25,
  enemyVehicle: 40,
  needMedic: 30,
  needAmmo: 30,
  orderAttack: 300,
  orderDefend: 300,
  orderMove: 300,
};
export const ATTENTION_LIMIT = 3;
export const isOrder = (k: MarkerKind): boolean => k.startsWith('order');

export interface Marker {
  id: number;
  owner: number;
  kind: MarkerKind;
  x: number;
  z: number;
  placedAt: number; // seconds
}

export class MarkerBook {
  readonly list: Marker[] = [];
  private nextId = 1;

  /** Places a marker; an Order needs the Officer role; the 4th Attention of one player replaces their oldest. */
  place(owner: number, kind: MarkerKind, x: number, z: number, now: number, isOfficer = false): Marker | null {
    if (isOrder(kind) && !isOfficer) return null;
    if (kind === 'attention') {
      const mine = this.list.filter((m) => m.owner === owner && m.kind === 'attention');
      if (mine.length >= ATTENTION_LIMIT) this.remove(mine[0]!.id);
    }
    const m: Marker = { id: this.nextId++, owner, kind, x, z, placedAt: now };
    this.list.push(m);
    return m;
  }

  remove(id: number): void {
    const i = this.list.findIndex((m) => m.id === id);
    if (i >= 0) this.list.splice(i, 1);
  }

  /** Drops expired markers; returns how many were removed. */
  expire(now: number): number {
    const before = this.list.length;
    for (let i = this.list.length - 1; i >= 0; i--) if (now - this.list[i]!.placedAt >= MARKER_TTL_S[this.list[i]!.kind]) this.list.splice(i, 1);
    return before - this.list.length;
  }
}

/** Remaining fraction for the countdown ring (1 → 0). */
export const markerRemaining = (m: Marker, now: number): number => Math.max(0, 1 - (now - m.placedAt) / MARKER_TTL_S[m.kind]);
