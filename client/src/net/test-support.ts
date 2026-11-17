// Test helpers for the net module (vitest only; not imported by the app): the real sim-wasm package
// and hand-built server C0 messages (the wasm exports client-side encoders only).
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import type { SimWasmModule } from './wasm';

const ROOT = fileURLToPath(new URL('../../../', import.meta.url));
export const PKG_DIR = `${ROOT}crates/sim-wasm/pkg/`;
export const HAVE_WASM = existsSync(`${PKG_DIR}frontline_sim_wasm_bg.wasm`);
export const WASM_HOW = 'wasm-pack build crates/sim-wasm --target web --profile wasm-release';

let cached: Promise<SimWasmModule> | undefined;
/** Loads the real wasm package synchronously-initialised (Node). */
export function loadWasm(): Promise<SimWasmModule> {
  cached ??= (async () => {
    const mod = (await import(/* @vite-ignore */ `${PKG_DIR}frontline_sim_wasm.js`)) as unknown as SimWasmModule & {
      initSync(o: { module: Uint8Array }): unknown;
    };
    mod.initSync({ module: readFileSync(`${PKG_DIR}frontline_sim_wasm_bg.wasm`) });
    mod.init();
    return mod;
  })();
  return cached;
}

class W {
  readonly b: number[] = [];
  u8(v: number): this {
    this.b.push(v & 255);
    return this;
  }
  u16(v: number): this {
    return this.u8(v >> 8).u8(v);
  }
  u32(v: number): this {
    return this.u16(v >>> 16).u16(v & 0xffff);
  }
  u64(v: bigint): this {
    return this.u32(Number((v >> 32n) & 0xffff_ffffn)).u32(Number(v & 0xffff_ffffn));
  }
  get bytes(): Uint8Array {
    return Uint8Array.from(this.b);
  }
}

export interface WelcomeFields {
  playerId?: number;
  serverTick?: number;
  tickHz?: number;
  matchSeed?: bigint;
  dataHash: bigint;
  mapId?: number;
  mode?: number;
  team?: number;
  clientTickOffset?: number;
}
export function welcomeBytes(f: WelcomeFields): Uint8Array {
  return new W()
    .u8(2)
    .u8(f.playerId ?? 1)
    .u32(f.serverTick ?? 100)
    .u8(f.tickHz ?? 30)
    .u64(f.matchSeed ?? 0x1234_5678_9abcn)
    .u64(f.dataHash)
    .u8(f.mapId ?? 0)
    .u8(f.mode ?? 0)
    .u8(f.team ?? 0)
    .u16(f.clientTickOffset ?? 3).bytes;
}
export const rejectBytes = (reason: number): Uint8Array => new W().u8(3).u8(reason).bytes;
export const pongBytes = (clientTime: number, serverTick: number, serverTimeMs = 0): Uint8Array => new W().u8(5).u32(clientTime).u32(serverTick).u32(serverTimeMs).bytes;
export const disconnectBytes = (reason: number): Uint8Array => new W().u8(6).u8(reason).bytes;

/** Reads the `client_time` out of a Ping C0 payload `[4][u32 BE]`. */
export function pingClientTime(b: Uint8Array): number {
  return (((b[1] as number) << 24) | ((b[2] as number) << 16) | ((b[3] as number) << 8) | (b[4] as number)) >>> 0;
}
