/**
 * Snapshot event segment -> bus events. The wasm decoder hands the events over as 16-byte records
 * (`id u16le, type u8, payload 13 B`; layout in crates/sim-wasm/src/snapshot.rs `pack_event`). The server
 * repeats every event in up to 3 packets, so records are de-duplicated by their 16-bit event id.
 */
import { LAYOUT_EVENT_RECORD_BYTES } from '../net/wasm';
import type { EventBus, GameEvents, Vec3 } from './event-bus';

export const EV_SHOT = 1;
export const EV_IMPACT = 2;
export const EV_HIT_CONFIRM = 3;
export const EV_DAMAGE_DIR = 4;
export const EV_EXPLOSION = 5;
export const EV_SUPPRESS = 6;
/** Explosion radius code: 1/8 m per step (crates/server/src/snapshot/events_out.rs `EXPLOSION_RADIUS_STEP_M`). */
export const EXPLOSION_RADIUS_STEP_M = 0.125;

/**
 * Metres per code of the 14-bit position fields of Shot / Impact. The protocol leaves their metric
 * meaning to the consumer (M1); M0 assumes the 256 m test map spans the full 14 bits (PLAN_ISSUES PI-023).
 */
export const EVENT_POS_STEP_M = 256 / 16384;
const TAU = Math.PI * 2;

const u16 = (b: Uint8Array, o: number): number => (b[o] as number) | ((b[o + 1] as number) << 8);

export class EventDecoder {
  private readonly seen = new Set<number>();
  private readonly order: number[] = [];
  private readonly muzzle: Vec3 = { x: 0, y: 0, z: 0 };
  private readonly dir: Vec3 = { x: 0, y: 0, z: 0 };
  private readonly pos: Vec3 = { x: 0, y: 0, z: 0 };
  private readonly shot: GameEvents['fire.remote'] = { shooter: 0, weapon: 0, muzzlePos: this.muzzle, dir: this.dir, distance: 0 };
  private readonly impact: GameEvents['impact'] = { pos: this.pos, normal: 0, material: 0 };
  private readonly hit: GameEvents['hit.confirm'] = { target: 0, zone: 0, damage: 0, killed: false };
  private readonly boom: GameEvents['explosion'] = { pos: this.pos, radius: 0, own: false };
  private readonly suppress: GameEvents['suppress'] = { angle: 0, intensity: 0 };
  private readonly dmg: GameEvents['damage.taken'] = { angle: 0, amount: 0 };
  /** Records dropped as duplicates (diagnostic). */
  duplicates = 0;
  /** Event ids remembered for de-duplication. */
  static readonly WINDOW = 128;

  constructor(
    private readonly bus: EventBus<GameEvents>,
    private readonly selfPos: () => Vec3,
  ) {}

  private firstTime(id: number): boolean {
    if (this.seen.has(id)) {
      this.duplicates++;
      return false;
    }
    this.seen.add(id);
    this.order.push(id);
    if (this.order.length > EventDecoder.WINDOW) this.seen.delete(this.order.shift() as number);
    return true;
  }

  reset(): void {
    this.seen.clear();
    this.order.length = 0;
  }

  /** Decodes and emits the event records of one snapshot. */
  decode(events: Uint8Array): void {
    const R = LAYOUT_EVENT_RECORD_BYTES;
    for (let o = 0; o + R <= events.length; o += R) {
      const id = u16(events, o);
      if (!this.firstTime(id)) continue;
      const type = events[o + 2] as number;
      const p = o + 3;
      switch (type) {
        case EV_SHOT: {
          const m = this.muzzle;
          m.x = u16(events, p + 2) * EVENT_POS_STEP_M;
          m.y = u16(events, p + 4) * EVENT_POS_STEP_M;
          m.z = u16(events, p + 6) * EVENT_POS_STEP_M;
          const yaw = ((u16(events, p + 8) & 0x3ff) / 1024) * TAU;
          const pitch = -Math.PI / 2 + ((u16(events, p + 10) & 0x1ff) / 511) * Math.PI;
          const cp = Math.cos(pitch);
          this.dir.x = Math.sin(yaw) * cp;
          this.dir.y = Math.sin(pitch);
          this.dir.z = Math.cos(yaw) * cp;
          const s = this.shot;
          s.shooter = events[p] as number;
          s.weapon = events[p + 1] as number;
          const me = this.selfPos();
          s.distance = Math.hypot(m.x - me.x, m.y - me.y, m.z - me.z);
          this.bus.emit('fire.remote', s);
          break;
        }
        case EV_IMPACT: {
          this.pos.x = u16(events, p) * EVENT_POS_STEP_M;
          this.pos.y = u16(events, p + 2) * EVENT_POS_STEP_M;
          this.pos.z = u16(events, p + 4) * EVENT_POS_STEP_M;
          this.impact.normal = u16(events, p + 6);
          this.impact.material = events[p + 8] as number;
          this.bus.emit('impact', this.impact);
          break;
        }
        case EV_HIT_CONFIRM: {
          this.hit.target = events[p] as number;
          this.hit.zone = events[p + 1] as number;
          this.hit.damage = (events[p + 2] as number) / 255;
          // The wire event carries no kill flag (kills arrive as a separate rules event, M1).
          this.hit.killed = false;
          this.bus.emit('hit.confirm', this.hit);
          break;
        }
        case EV_DAMAGE_DIR: {
          this.dmg.angle = ((events[p] as number) / 256) * TAU;
          this.dmg.amount = (events[p + 1] as number) / 255;
          this.bus.emit('damage.taken', this.dmg);
          break;
        }
        case EV_EXPLOSION: {
          this.pos.x = u16(events, p) * EVENT_POS_STEP_M;
          this.pos.y = u16(events, p + 2) * EVENT_POS_STEP_M;
          this.pos.z = u16(events, p + 4) * EVENT_POS_STEP_M;
          // payload byte p + 6 is the explosive kind; the radius is what the consumers need.
          this.boom.radius = (events[p + 7] as number) * EXPLOSION_RADIUS_STEP_M;
          this.bus.emit('explosion', this.boom);
          break;
        }
        case EV_SUPPRESS: {
          this.suppress.angle = ((events[p] as number) / 256) * TAU;
          this.suppress.intensity = (events[p + 1] as number) / 255;
          this.bus.emit('suppress', this.suppress);
          break;
        }
        default:
          break; // unknown type: ignore (forward compatible)
      }
    }
  }
}
