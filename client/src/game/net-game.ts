/**
 * Networked-play composition (M0-022): sim-wasm load, connection, collision chunks, input, `GameLoop`.
 * Started by `main.ts` when the URL has `?server=host:port` (see `net/config.ts` for the other options).
 *
 * Wiring points for the render side (M0-025): `game.state` (local pose, remote entity SoA, signals) and the
 * event bus (`fire.local`, `fire.remote`, `impact`, `hit.confirm`, `damage.taken`, `entity.*`,
 * `snapshot.applied`). Nothing here touches render / ui / audio.
 */
import { InputSampler, PointerLockController } from '../input';
import { connect, parseNetConfig, type NetConfig, type Session } from '../net';
import { assertLayoutMatches, type SimWasmModule } from '../net/wasm';
import { EventBus, type GameEvents } from './event-bus';
import { FixedStepper } from './fixed-step';
import { GameLoop } from './game-loop';
import { GameState } from './state/game-state';

export interface NetGameHandle {
  readonly game: GameLoop;
  readonly gameState: GameState;
  readonly session: Session;
  readonly sampler: InputSampler;
  /** The loaded sim-wasm module (data queries for the render side: roleDef / weaponDef / weaponIndex). */
  readonly wasm: SimWasmModule;
  /** Called by the render loop with the steps granted by its `FixedStepper`. */
  update(steps: number, alpha: number, dtMs: number): void;
  overlayLines(): string[];
  dispose(): void;
}

interface WasmPkg extends SimWasmModule {
  default(opts: { module_or_path: string }): Promise<unknown>;
}

export async function loadWasm(base: string): Promise<SimWasmModule> {
  const mod = (await import(/* @vite-ignore */ `${base}wasm/frontline_sim_wasm.js`)) as WasmPkg;
  await mod.default({ module_or_path: `${base}wasm/frontline_sim_wasm_bg.wasm` });
  mod.init();
  assertLayoutMatches(mod);
  return mod;
}

async function loadChunks(base: string, mapId: string): Promise<{ bx: number; bz: number; data: Uint8Array }[]> {
  const jobs: Promise<{ bx: number; bz: number; data: Uint8Array }>[] = [];
  for (let bz = 0; bz < 4; bz++) {
    for (let bx = 0; bx < 4; bx++) {
      jobs.push(
        fetch(`${base}maps/${mapId}/chunks/c_${bx}_${bz}.bin`).then(async (r) => {
          if (!r.ok) throw new Error(`collision chunk c_${bx}_${bz}: HTTP ${r.status} (run \`pnpm -C client sync-map\`)`);
          return { bx, bz, data: new Uint8Array(await r.arrayBuffer()) };
        }),
      );
    }
  }
  return Promise.all(jobs);
}

export async function startNetGame(
  canvas: HTMLCanvasElement,
  bus: EventBus<GameEvents>,
  baseUrl = import.meta.env.BASE_URL,
  onDispose?: () => void,
  join?: { config: NetConfig; ticket: Uint8Array },
): Promise<NetGameHandle> {
  const [wasm, chunks] = await Promise.all([loadWasm(baseUrl), loadChunks(baseUrl, 'm0')]);
  const config = join?.config ?? parseNetConfig(location.search, location);
  const gameState = new GameState(bus);
  const session = await connect({
    config, wasm, bus,
    ...(join ? { hello: { ticket: join.ticket } } : {}),
    onC4Event: (event) => gameState.apply(event),
    onDeployResult: (result) => gameState.applyDeployResult(result),
  });
  const w = session.welcome;
  const sampler = new InputSampler();
  try {
    const sens = Number(localStorage.getItem('frontline.sens'));
    if (Number.isFinite(sens) && sens > 0) sampler.sensitivity = sens;
  } catch {
    /* storage unavailable: default sensitivity */
  }
  const lock = new PointerLockController(sampler, canvas, document, window);
  lock.attach();
  const game = new GameLoop({ wasm, decoder: new wasm.SnapshotDecoder(), bus, net: session, input: sampler, matchSeed: w.matchSeed, mapId: w.mapId, playerId: w.playerId });
  game.loadCollision(chunks);
  const offSnap = session.onSnapshot((bytes) => game.onSnapshot(bytes));

  // A hidden tab gets no animation frames: keep stepping from a timer so inputs and the connection stay alive
  // (browsers throttle background timers, so this is best effort; see ARCHITECTURE section 20 #6).
  const bgStepper = new FixedStepper();
  let bgTimer: ReturnType<typeof setInterval> | undefined;
  let bgLast = 0;
  const onVisibility = (): void => {
    if (document.hidden) {
      game.suspendInput();
      bgStepper.reset();
      bgLast = performance.now();
      bgTimer = setInterval(() => {
        const now = performance.now();
        const dt = now - bgLast;
        bgLast = now;
        const { steps, alpha } = bgStepper.advance(dt);
        game.update(steps, alpha, dt);
      }, 1000 / 30);
    } else if (bgTimer !== undefined) {
      clearInterval(bgTimer);
      bgTimer = undefined;
    }
  };
  document.addEventListener('visibilitychange', onVisibility);

  return {
    game,
    gameState,
    session,
    sampler,
    wasm,
    update: (steps, alpha, dtMs) => {
      if (!document.hidden) game.update(steps, alpha, dtMs);
    },
    overlayLines: () => {
      const n = game.state.net;
      const st = session.stats();
      const rs = game.reconciler.stats;
      return [
        `net ${st.transport} rtt ${st.rttMs.toFixed(0)} ms jitter ${st.jitterMs.toFixed(1)} ms | up ${n.upKbps.value.toFixed(1)} kbps`,
        `reconcile corrections ${rs.corrections} snaps ${rs.snaps} lastErr ${rs.lastErrM.toFixed(3)} m maxErr ${rs.maxErrM.toFixed(3)} m (kept ${rs.toleranceKept})`,
        `inputs ring ${game.ring.length}/${game.ring.capacity} lead ${session.clock.inputLeadTicks()} t ticks +${game.planner.extras}/-${game.planner.skips} | entities ${game.state.entities.count}`,
      ];
    },
    dispose: () => {
      onDispose?.();
      offSnap();
      lock.detach();
      document.removeEventListener('visibilitychange', onVisibility);
      if (bgTimer !== undefined) clearInterval(bgTimer);
      session.close();
      game.dispose();
    },
  };
}
