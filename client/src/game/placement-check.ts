/** Adapter from placement facts to the real sim-wasm check. Gameplay rules live in sim. */
import type { WasmPredictor } from '../net/wasm';
import { PLACE_ERROR_KEY, placementNext, type PlacementKind, type PlacementState } from './placement';

export type PlacementPoint = readonly [number, number, number];
type Side = 'us' | 'ger';
interface ObjectiveFact {
  readonly id: number;
  readonly pos: PlacementPoint;
  readonly radius_m: number;
  readonly owner: Side | null;
}

/** Read-only facts for the current player and nearby/known world. Absence is represented by
 * a null snapshot, since an empty list represents a known empty list. Enemies must come from
 * the client's filtered view. Only the server knows the full authoritative state. */
export type PlacementFacts =
  | {
    readonly kind: 'garrison';
    readonly alive: boolean;
    readonly officer: boolean;
    readonly team: Side;
    readonly hq: readonly PlacementPoint[];
    readonly garrisons: readonly (readonly [Side, PlacementPoint])[];
    readonly objectives: readonly ObjectiveFact[];
    readonly enemies: readonly PlacementPoint[];
    readonly available_supplies: number;
  }
  | {
    readonly kind: 'supply_crate';
    readonly alive: boolean;
    readonly support: boolean;
    readonly carrying: boolean;
    readonly team: Side;
    readonly objectives: readonly ObjectiveFact[];
  }
  | {
    readonly kind: 'ammo_crate';
    readonly alive: boolean;
    readonly support: boolean;
    readonly tick: number;
    readonly used: number;
    readonly last_placed_tick: number | null;
    readonly crates: readonly PlacementPoint[];
  };

export interface PlacementWasm {
  placement_check(predictor: WasmPredictor, kind: PlacementKind, pos: Float32Array, factsJson: string):
    { legal: true; supplyCost?: number; remaining?: number } | { legal: false; reason: string };
}

const REASON_KEYS: Readonly<Record<string, string>> = {
  ...PLACE_ERROR_KEY,
  NotSupport: 'ui.officer.tool.needSupport',
  NotCarrying: 'ui.officer.err.notCarrying',
  LimitPerLife: 'ui.officer.err.ammoLimit',
  Cooldown: 'ui.officer.err.ammoCooldown',
  Stacked: 'ui.officer.err.ammoStacked',
};
const PREVIEW_UNAVAILABLE = 'ui.officer.err.previewUnavailable';

export function refreshPlacement(
  wasm: Partial<PlacementWasm>,
  predictor: WasmPredictor,
  state: PlacementState,
  point: PlacementPoint | null,
  facts: PlacementFacts | null,
): PlacementState {
  if (state.mode !== 'placing' && state.mode !== 'invalid') return state;
  if (point?.every(Number.isFinite) && facts?.kind === state.kind && wasm.placement_check) {
    const result = wasm.placement_check(predictor, facts.kind, new Float32Array(point), JSON.stringify(facts));
    if (result.legal) return placementNext(state, { t: 'check', result });
    if (Object.hasOwn(REASON_KEYS, result.reason)) {
      return placementNext(state, { t: 'check', result: { legal: false, reasonKey: REASON_KEYS[result.reason] } });
    }
  }
  return placementNext(state, { t: 'check', result: { legal: false, reasonKey: PREVIEW_UNAVAILABLE } });
}
