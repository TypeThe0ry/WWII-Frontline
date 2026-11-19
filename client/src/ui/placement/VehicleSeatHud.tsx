import type { SeatCommand, VehicleSeat } from '../../game/placement-actions';
import type { SeatState } from '../../game/seat';

export interface VehicleSeatRow {
  seat: VehicleSeat;
  occupied: boolean;
  occupantName?: string;
}

const names: Record<VehicleSeat, string> = { driver: 'Driver', gunner: 'Gunner', commander: 'Commander' };

function SeatList({ seats, selected, pending, choose }: {
  seats: readonly VehicleSeatRow[];
  selected?: SeatState['seat'];
  pending?: boolean;
  choose?: (seat: VehicleSeat) => void;
}) {
  return (
    <ol aria-label="Vehicle seats">
      {seats.map(({ seat, occupied, occupantName }) => (
        <li key={seat} aria-current={seat === selected ? 'true' : undefined}>
          {names[seat]}{' — '}{seat === selected ? 'You' : occupied ? occupantName ?? 'Occupied' : 'Empty'}
          {choose && <button type="button" aria-label={`Select ${names[seat]}`} disabled={pending || occupied || seat === selected} onClick={() => choose(seat)}>Select</button>}
        </li>
      ))}
    </ol>
  );
}

export interface VehicleEnterPromptProps {
  vehicleId: number;
  name: string;
  seats: readonly VehicleSeatRow[];
  canBoardNearest: boolean;
  menuOpen?: boolean;
  pending?: boolean;
  /** Omit while the transport has no targeted seat command. */
  onSeatCommand?: (command: SeatCommand) => void;
}

export function VehicleEnterPrompt({ vehicleId, name, seats, canBoardNearest, menuOpen, pending, onSeatCommand }: VehicleEnterPromptProps) {
  return (
    <section aria-label={`Board ${name}`}>
      <b>{name}</b>
      <p role="status">{pending ? 'Boarding…' : canBoardNearest ? 'F: Enter nearest empty seat · Hold F: View seats' : 'Boarding unavailable'}</p>
      <SeatList seats={seats} pending={pending} choose={menuOpen && onSeatCommand ? (seat) => onSeatCommand({ kind: 'enter', vehicleId, seat }) : undefined} />
    </section>
  );
}

export interface VehicleSeatHudProps {
  state: SeatState;
  seats: readonly VehicleSeatRow[];
  speedKmh?: number;
  mainAmmo?: { type: string; remaining: number };
  coaxRemaining?: number;
  tick?: number;
  tickHz?: number;
  /** Host-computed progress from the current exit hold, 0..1. */
  exitProgress?: number;
  bailout?: boolean;
  /** Omit while the transport has no targeted seat command. */
  onSeatCommand?: (command: SeatCommand) => void;
}

export function VehicleSeatHud({ state, seats, speedKmh, mainAmmo, coaxRemaining, tick, tickHz, exitProgress, bailout, onSeatCommand }: VehicleSeatHudProps) {
  if (bailout) return <p role="alert">Bailout: Vehicle destroyed</p>;
  const { seat, vehicleId, pending, target, readyAt } = state;
  if (seat === 'none') return null;
  const remaining = readyAt !== undefined && tick !== undefined && tickHz && tickHz > 0 ? Math.max(0, (readyAt - tick) / tickHz) : 0;
  const choose = onSeatCommand && vehicleId !== undefined ? (seat: VehicleSeat) => onSeatCommand({ kind: 'swap', vehicleId, seat }) : undefined;
  return (
    <section aria-label="Vehicle HUD">
      <SeatList seats={seats} selected={seat} pending={pending !== undefined} choose={choose} />
      {choose && <p>Number keys: Change seat</p>}
      {speedKmh !== undefined && <div>Speed: {Math.round(speedKmh)} km/h</div>}
      <div>Main gun: {mainAmmo ? `${mainAmmo.type} · ${mainAmmo.remaining}` : '—'}</div>
      <div>Coax: {coaxRemaining ?? '—'}</div>
      {pending === 'swap' && target && target !== 'none' && (
        <p role="status">Switching to {names[target]}{remaining > 0 ? ` · ${remaining.toFixed(1)} s` : ' · Awaiting confirmation'}</p>
      )}
      <p>{pending === 'exit' ? 'Exiting…' : 'Hold F to exit'}</p>
      {exitProgress !== undefined && exitProgress > 0 && <progress aria-label="Exit hold" max={1} value={Math.min(exitProgress, 1)} />}
    </section>
  );
}
