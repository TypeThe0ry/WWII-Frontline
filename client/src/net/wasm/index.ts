// Thin TS surface over the sim-wasm package (M0-014). Transport / clock / reconciliation: M0-021/022.
// The generated package lives in crates/sim-wasm/pkg (git-ignored; `wasm-pack build crates/sim-wasm
// --target web --profile wasm-release`). It is injected rather than imported statically so that a
// clean checkout without the wasm build still type-checks and bundles; M0-022 wires the real import.
import * as L from './layout';
import type { SimWasmModule } from './types';

export * from './layout';
export type * from './types';
export { L as layout };

export interface DecodedSnapshot {
  tick: number;
  ackInputSeq: number;
  inputSlack: number;
  self: Float32Array;
  ids: Uint8Array;
  pos: Float32Array;
  yaw: Float32Array;
  pitch: Float32Array;
  anim: Uint8Array;
  stance: Uint8Array;
  weapon: Uint8Array;
  flags: Uint8Array;
  health: Uint8Array;
  removed: Uint8Array;
  events: Uint8Array;
}

/** Decodes with the wasm decoder and returns views (valid until the next decode). */
export function decodeSnapshot(dec: InstanceType<SimWasmModule['SnapshotDecoder']>, bytes: Uint8Array): DecodedSnapshot {
  const tick = dec.decode(bytes);
  return {
    tick,
    ackInputSeq: dec.ackInputSeq(),
    inputSlack: dec.inputSlack(),
    self: dec.selfState(),
    ids: dec.ids(),
    pos: dec.pos(),
    yaw: dec.yaw(),
    pitch: dec.pitch(),
    anim: dec.anim(),
    stance: dec.stance(),
    weapon: dec.weapon(),
    flags: dec.flags(),
    health: dec.health(),
    removed: dec.removed(),
    events: dec.events(),
  };
}

/** Throws when the TS constants and the loaded wasm build disagree (stale layout.ts). */
export function assertLayoutMatches(wasm: SimWasmModule): void {
  const live = JSON.parse(wasm.layoutJson()) as Record<string, number>;
  const mine = L as unknown as Record<string, number>;
  for (const [k, v] of Object.entries(live)) {
    if (mine[`LAYOUT_${k}`] !== v) throw new Error(`layout mismatch for ${k}: wasm ${v}, ts ${String(mine[`LAYOUT_${k}`])}`);
  }
}
