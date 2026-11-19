import { signal } from '@preact/signals';
import type { ReadonlySignal } from '@preact/signals';
import { BUTTONS } from '../input/buttons';
import { fKeyAction, PLACEMENT_IDLE, placementNext, seatForKey } from './placement';
import type { PlacementKind, PlacementResult, PlacementState } from './placement';
import type { PlacementPoint } from './placement-check';
import type { Seat, SeatMachine } from './seat';

/** Host adapter, normally `(state, point) => game.checkPlacement(state, point, visibleFacts)`. */
export type PlacementChecker = (state: PlacementState, point: PlacementPoint) => PlacementState;
const unavailable: PlacementResult = { legal: false, reasonKey: 'ui.officer.err.previewUnavailable' };

export type PlacementCommand =
  | { kind: 'garrison'; pos: PlacementPoint }
  // Crates are dropped/placed at the local player's feet by the sim, never at a remote ray hit.
  | { kind: 'supply_crate' | 'ammo_crate' };
export type PlacementReply = { ok: true } | { ok: false; reasonKey: string };
export type PlacementSender = (command: PlacementCommand) => Promise<PlacementReply>;

export interface PlacementView {
  readonly state: PlacementState;
  readonly target: PlacementPoint | null;
  readonly preview: PlacementResult | null;
}

const idle: PlacementView = { state: PLACEMENT_IDLE, target: null, preview: null };

/** Local preview and command lifecycle only. The host supplies ray/feet positions, state and transport. */
export class PlacementActions {
  readonly #view = signal<PlacementView>(idle);
  readonly view: ReadonlySignal<PlacementView> = this.#view;

  constructor(private readonly checker?: PlacementChecker, private readonly send?: PlacementSender) {}

  enter(kind: PlacementKind): void {
    if (this.#view.peek().state.mode === 'idle') {
      this.#view.value = { ...idle, state: placementNext(idle.state, { t: 'enter', kind }) };
    }
  }

  /** Pass null for no target, a ray hit for Garrison, and the player position for crates. */
  update(target: PlacementPoint | null): void {
    const current = this.#view.peek();
    const { state } = current;
    if (!state.kind || state.mode === 'committing') return;
    // Keep a server rejection visible on the rejected target, rather than clearing it every frame.
    const sameTarget = target && current.target && target.every((v, i) => v === current.target?.[i]);
    if (state.mode === 'placing' && state.reasonKey && !state.result && sameTarget) return;
    const checked = target?.every((v) => Number.isFinite(Math.fround(v))) && this.checker
      ? this.checker(state, target)
      : placementNext(state, { t: 'check', result: unavailable });
    this.#view.value = { state: checked, target: target && [...target], preview: checked.result ?? null };
  }

  cancel(): void {
    if (this.#view.peek().state.mode !== 'committing') this.#view.value = idle;
  }

  /** A missing sender dispatches nothing. Only a positive preview can produce an intent. */
  async commit(): Promise<PlacementReply | undefined> {
    if (!this.send) return;
    this.update(this.#view.peek().target);
    const current = this.#view.peek();
    const { state, target, preview } = current;
    if (state.mode !== 'placing' || !state.kind || !target || !preview?.legal) return;
    const committing = placementNext(state, { t: 'commit' });
    this.#view.value = { ...current, state: committing };
    const kind = state.kind;
    let reply: PlacementReply;
    try {
      reply = await this.send(kind === 'garrison' ? { kind, pos: target } : { kind });
    } catch (error) {
      // No authoritative rejection reason on a transport failure. Do not retry automatically.
      this.#view.value = { ...current, state, preview: unavailable };
      throw error;
    }
    if (reply.ok) {
      this.#view.value = { ...idle, state: placementNext(committing, { t: 'accepted' }) };
    } else {
      const rejected = placementNext(committing, { t: 'rejected', reasonKey: reply.reasonKey });
      this.#view.value = { ...current, state: rejected, preview: { ...preview, legal: false, reasonKey: reply.reasonKey } };
    }
    return reply;
  }
}

export interface SeatInputSample {
  /** The host resolves whether the current target qualifies for the server's nearest-board path. */
  canBoard: boolean;
  inVehicle: boolean;
  fDown: boolean;
  /** Total hold duration, including on the release sample. */
  heldS: number;
}

/**
 * Sample once per input tick. Owns VEHICLE_SEAT; claims USE only for a press routed to a vehicle.
 * Board on release (one C1 tick), open the menu once per hold, and send the exit level from the
 * start of a seated press so the server, not a second client timer, owns the 0.8 s exit deadline.
 */
export class SeatInputAdapter {
  #press: 'interact' | 'board' | 'exit' | undefined;
  #menuOpened = false;

  reset(): void {
    this.#press = undefined;
    this.#menuOpened = false;
  }

  sample(buttons: number, input: SeatInputSample): { buttons: number; openSeatMenu: boolean } {
    buttons &= ~BUTTONS.VEHICLE_SEAT;
    if (input.fDown && !this.#press) {
      this.#press = input.inVehicle ? 'exit' : input.canBoard ? 'board' : 'interact';
    }
    const press = this.#press;
    if (press === 'exit' || press === 'board') buttons &= ~BUTTONS.USE;
    let openSeatMenu = false;
    if (press === 'exit' && input.inVehicle && input.fDown) buttons |= BUTTONS.VEHICLE_SEAT;
    if (press === 'board' && !input.inVehicle && input.canBoard && !this.#menuOpened) {
      const action = fKeyAction(false, input.heldS, !input.fDown);
      if (action === 'enterNearest') buttons |= BUTTONS.VEHICLE_SEAT;
      if (action === 'seatMenu') openSeatMenu = this.#menuOpened = true;
    }
    if (!input.fDown) this.reset();
    return { buttons, openSeatMenu };
  }
}

export type VehicleSeat = Exclude<Seat, 'none'>;
export type SeatCommand = { kind: 'enter' | 'swap'; vehicleId: number; seat: VehicleSeat };
export type SquadToolCommand =
  | { kind: 'lock'; locked: boolean }
  | { kind: 'kick' | 'promote'; playerId: number };

/** SquadOp C0 payload (without transport framing). Roster events, not dispatch, confirm changes. */
export function encodeSquadToolCommand(command: SquadToolCommand): Uint8Array {
  const squadOp = 0x12; // crates/protocol/src/channel.rs::c0_type::SQUAD_OP
  if (command.kind === 'lock') return Uint8Array.of(squadOp, command.locked ? 7 : 8, 0, 0, 0);
  const target = command.playerId;
  if (!Number.isInteger(target) || target < 0 || target > 127) throw new RangeError('Invalid squad member ID');
  // SquadOpKind kick/promote; current squad and empty name key.
  return Uint8Array.of(squadOp, command.kind === 'kick' ? 4 : 5, 0, target, 0);
}

/** Call only with a real targeted-seat transport. The current C1 button cannot encode this intent. */
export function requestSeatSwapForKey(
  input: { key: string; seats: readonly VehicleSeat[]; tick: number; speedKmh: number },
  machine: SeatMachine,
  send?: (intent: SeatCommand) => void,
): boolean {
  const seat = seatForKey(input.key, input.seats);
  const { vehicleId } = machine.state;
  if (!send || !seat || vehicleId === undefined || !machine.requestSwap(seat, input.tick, input.speedKmh)) return false;
  try {
    send({ kind: 'swap', vehicleId, seat });
  } catch (error) {
    machine.reject();
    throw error;
  }
  return true;
}
