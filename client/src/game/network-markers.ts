import type { LegacyTeamMarker, MarkerPosition, TeamMarkersFrame } from '../net/comms';

export interface NetworkMarker extends MarkerPosition {
  readonly id: number;
  /** The high bit distinguishes a manual TeamMarkerKind from a PingKind. */
  readonly kind: number;
  readonly owner: number;
  /** Present only for a Thanks ping with a server-verified friendly target. */
  readonly target?: number;
  readonly expiresAtMs: number;
}

const UNTIL_REVOKED_TTL = 255;
const newer16 = (value: number, previous: number): boolean => {
  const diff = (value - previous) & 0xffff;
  return diff !== 0 && diff < 0x8000;
};

export class NetworkMarkers {
  private readonly active = new Map<number, NetworkMarker>();
  // Keep the latest ordering stamp for each wire ID through the whole session, even after
  // removal. A delayed datagram may outlive the marker or the C2 retransmission window.
  private readonly order = new Map<number, { tick: number; seq: number }>();
  private legacyTick: number | undefined;
  private legacy: (LegacyTeamMarker & { expiresAtMs: number })[] = [];

  get lifecycle(): ReadonlyMap<number, NetworkMarker> { return this.active; }
  /** C3 fallback is a separate projection: legacy records have no stable ID. */
  get legacyProjection(): readonly LegacyTeamMarker[] { return this.legacy; }

  private accept(id: number, tick: number, seq: number): boolean {
    const last = this.order.get(id);
    if (last) {
      // The scheduler supersedes queued lifecycle events for this recipient and marker.
      // Packet tick orders in-flight events. Event sequence breaks ties within a packet.
      const delta = (tick - last.tick) | 0;
      if (delta < 0 || (delta === 0 && !newer16(seq, last.seq))) return false;
    }
    this.order.set(id, { tick, seq });
    return true;
  }

  upsert(marker: Omit<NetworkMarker, 'expiresAtMs'>, tick: number, seq: number, nowMs: number): void {
    if (!this.accept(marker.id, tick, seq)) return;
    const expiresAtMs = marker.ttl === UNTIL_REVOKED_TTL ? Infinity : nowMs + marker.ttl * 1000;
    this.active.set(marker.id, { ...marker, expiresAtMs });
  }

  remove(id: number, tick: number, seq: number): void {
    if (this.accept(id, tick, seq)) this.active.delete(id);
  }

  applyLegacy(frame: TeamMarkersFrame, nowMs: number): void {
    if (this.legacyTick !== undefined && !newer16(frame.tick, this.legacyTick)) return;
    this.legacyTick = frame.tick;
    this.legacy = frame.markers.map((marker) => ({ ...marker, expiresAtMs: nowMs + marker.ttl * 1000 }));
  }

  expire(nowMs: number): void {
    for (const [id, marker] of this.active) {
      if (marker.expiresAtMs <= nowMs) this.active.delete(id);
    }
    if (this.legacy.some((marker) => marker.expiresAtMs <= nowMs)) {
      this.legacy = this.legacy.filter((marker) => marker.expiresAtMs > nowMs);
    }
  }

  discardThroughTeamChange(tick: number): void {
    // A new-team C2/C3 frame can arrive before the reliable C4 move notice.
    for (const [id, stamp] of this.order) {
      if (((stamp.tick - tick) | 0) <= 0) {
        this.active.delete(id);
        this.order.delete(id);
      }
    }
    const tick16 = tick & 0xffff;
    if (this.legacyTick === undefined || !newer16(this.legacyTick, tick16)) {
      this.legacy = [];
      this.legacyTick = tick16;
    }
  }

  reset(): void {
    this.active.clear();
    this.order.clear();
    this.legacyTick = undefined;
    this.legacy = [];
  }
}
