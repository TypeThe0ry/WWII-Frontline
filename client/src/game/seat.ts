/**
 * Vehicle seat state (M1-041, GAMEDESIGN 14.7). The server is authoritative: a request puts the
 * machine in `pending` ("requesting..." in the UI) until `confirm` / `reject` arrives. Swapping
 * seats is immediate below 5 km/h and takes 2 s (60 ticks) while moving.
 */
export type Seat = 'none' | 'driver' | 'gunner' | 'commander';
export type SeatPending = 'enter' | 'exit' | 'swap';

export interface SeatState {
  seat: Seat;
  vehicleId?: number;
  pending?: SeatPending;
  /** Seat the pending request asks for. */
  target?: Seat;
  /** Tick the request may take effect (swap while moving). */
  readyAt?: number;
}

export const SWAP_STOPPED_KMH = 5;
export const SWAP_MOVING_TICKS = 60;

export class SeatMachine {
  state: SeatState = { seat: 'none' };
  /** Requests refused locally or by the server (diagnostic). */
  refused = 0;

  /** Ticks until a swap may complete at `speedKmh`. */
  static swapDelay(speedKmh: number): number {
    return Math.abs(speedKmh) < SWAP_STOPPED_KMH ? 0 : SWAP_MOVING_TICKS;
  }

  /** Returns true when the request should be sent to the server. */
  requestEnter(vehicleId: number, seat: Exclude<Seat, 'none'>): boolean {
    if (this.state.seat !== 'none' || this.state.pending) return this.refuse();
    this.state = { seat: 'none', vehicleId, pending: 'enter', target: seat };
    return true;
  }

  requestExit(): boolean {
    if (this.state.seat === 'none' || this.state.pending === 'exit') return this.refuse();
    this.state = { ...this.state, pending: 'exit', target: 'none' };
    return true;
  }

  requestSwap(seat: Exclude<Seat, 'none'>, tick: number, speedKmh: number): boolean {
    const s = this.state;
    if (s.seat === 'none' || s.pending !== undefined || s.seat === seat) return this.refuse();
    this.state = { ...s, pending: 'swap', target: seat, readyAt: tick + SeatMachine.swapDelay(speedKmh) };
    return true;
  }

  /** True once a pending swap's delay has elapsed (the request is then sent / may complete). */
  due(tick: number): boolean {
    const s = this.state;
    return s.pending === 'swap' && (s.readyAt === undefined || tick >= s.readyAt);
  }

  /** Server confirmed the seat (also used for server-initiated changes, e.g. the vehicle died). */
  confirm(seat: Seat, vehicleId?: number): void {
    this.state = seat === 'none' ? { seat: 'none' } : { seat, vehicleId };
  }

  /** Server refused the pending request: back to the previous seat. */
  reject(): void {
    const { pending, target, readyAt, ...rest } = this.state;
    void pending; void target; void readyAt;
    this.refused++;
    this.state = rest.seat === 'none' ? { seat: 'none' } : { seat: rest.seat, vehicleId: rest.vehicleId };
  }

  /** The local player runs the vehicle predictor only as driver; the character KCC otherwise. */
  get predictsVehicle(): boolean {
    return this.state.seat === 'driver';
  }

  private refuse(): false {
    this.refused++;
    return false;
  }
}
