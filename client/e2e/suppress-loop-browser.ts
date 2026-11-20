import { attachGameAudio } from '../src/audio/game-audio';
import { createDeployActions, HQ_SPAWN_ID, type DeployRequest } from '../src/game/deploy-actions';
import { EventBus, type GameEvents } from '../src/game/event-bus';
import { FixedStepper } from '../src/game/fixed-step';
import { startNetGame } from '../src/game/net-game';
import { SettingsStore } from '../src/game/settings';
import type { C4Event } from '../src/game/state/c4';
import { Persist } from '../src/input/bindings';
import { BUTTONS } from '../src/input/buttons';
import { mountMatchUi } from '../src/match-ui';

async function join(withHud: boolean) {
  const canvas = document.querySelector('canvas')!;
  const gl = withHud ? canvas.getContext('webgl2') : null;
  const ext = gl?.getExtension('WEBGL_debug_renderer_info');
  const renderer = ext && gl?.getParameter(ext.UNMASKED_RENDERER_WEBGL) as string | undefined;
  const bus = new EventBus<GameEvents>();
  const net = await startNetGame(canvas, bus);
  const { game, sampler, session, wasm } = net;
  // The browser binding for throw is pending. Add its existing C1 bit at the input boundary.
  let pulseGrenade = false;
  const build = sampler.build.bind(sampler);
  sampler.build = (...args) => {
    const cmd = build(...args);
    if (pulseGrenade) {
      cmd.buttons |= BUTTONS.GADGET;
      pulseGrenade = false;
    }
    return cmd;
  };
  const actions = createDeployActions(session.transport, wasm);
  const c4: C4Event[] = [];
  const offC4 = session.transport.onC4((bytes) => {
    c4.push(JSON.parse(wasm.decodeC4(bytes)) as C4Event);
  });
  const suppress: number[] = [];
  const offSuppress = bus.on('suppress', ({ intensity }) => suppress.push(intensity));
  const hits: { target: number; zone: number; damage: number }[] = [];
  const offHit = bus.on('hit.confirm', ({ target, zone, damage }) => hits.push({ target, zone, damage }));
  const blasts: { x: number; z: number; radius: number }[] = [];
  const offBlast = bus.on('explosion', ({ pos, radius }) => blasts.push({ x: pos.x, z: pos.z, radius }));
  const frameCallbacks = new Set<() => void>();
  const persist = new Persist(() => localStorage);
  const offUi = withHud && mountMatchUi({ canvas, game: net, bus, persist, settings: new SettingsStore(persist),
    onFrame: (fn) => { frameCallbacks.add(fn); return () => frameCallbacks.delete(fn); }, applyFov: () => {}, onLeave: () => {} });

  // Observe the native graph created by attachGameAudio. Calls and automation still execute in Chromium.
  const filters: BiquadFilterNode[] = [];
  const gains: GainNode[] = [];
  const nativeFilter = AudioContext.prototype.createBiquadFilter;
  const nativeGain = AudioContext.prototype.createGain;
  const gameAudio = withHud && (() => {
    AudioContext.prototype.createBiquadFilter = function (this: AudioContext) {
      const node = nativeFilter.call(this);
      filters.push(node);
      return node;
    };
    AudioContext.prototype.createGain = function (this: AudioContext) {
      const node = nativeGain.call(this);
      gains.push(node);
      return node;
    };
    try { return attachGameAudio(bus, { listener: () => ({ x: 0, y: 0, z: 0 }), weaponId: () => undefined }); }
    finally { AudioContext.prototype.createBiquadFilter = nativeFilter; AudioContext.prototype.createGain = nativeGain; }
  })();
  const audio = () => ({ context: filters[0]?.context.state, cutoff: filters[0]?.frequency.value, gain: gains[0]?.gain.value });

  const stepper = new FixedStepper();
  let last = performance.now();
  let frame = 0;
  const update = (now: number) => {
    const dt = now - last;
    last = now;
    const { steps, alpha } = stepper.advance(dt);
    net.update(steps, alpha, dt);
    if (gameAudio) gameAudio.flush();
    for (const fn of frameCallbacks) fn();
    if (gl) gl.clear(gl.COLOR_BUFFER_BIT);
    frame = requestAnimationFrame(update);
  };
  frame = requestAnimationFrame(update);
  window.addEventListener('pagehide', () => {
    cancelAnimationFrame(frame);
    if (offUi) offUi();
    if (gameAudio) gameAudio.dispose();
    offHit(); offBlast(); offSuppress(); offC4(); actions.dispose(); net.dispose();
  }, { once: true });

  return {
    id: session.welcome.playerId, team: session.welcome.team, transport: session.kind, renderer,
    health: () => game.state.net.health.peek(),
    tick: () => game.state.net.snapshotTick.peek(),
    pose: () => ({ x: game.state.local.simPos[0]!, y: game.state.local.simPos[1]!, z: game.state.local.simPos[2]! }),
    diag: () => ({ shots: game.stats.fireLocal, ammo: game.state.net.ammoAuth.peek(), yaw: game.state.local.yaw, pitch: game.state.local.pitch, hits }),
    c4: () => c4, suppress: () => suppress, blasts: () => blasts, audio,
    roster: () => net.gameState.roster.list(),
    createSquad: () => actions.createSquad('infantry', 'Alpha'),
    joinSquad: (id: number) => actions.joinSquad(id),
    deploy: (req: Omit<DeployRequest, 'spawnId'>) => actions.deploy({ ...req, spawnId: HQ_SPAWN_ID }),
    aim: (yaw: number, pitch: number) => sampler.setView(yaw, pitch),
    walkTo(x: number, z: number) {
      return new Promise<void>((resolve, reject) => {
        const deadline = performance.now() + 6000;
        const next = () => {
          const dx = x - game.state.local.simPos[0]!, dz = z - game.state.local.simPos[2]!;
          if (Math.hypot(dx, dz) <= 0.35) { sampler.release('KeyW'); resolve(); }
          else if (performance.now() > deadline) { sampler.release('KeyW'); reject(new Error('arena walk timed out')); }
          else { sampler.setView(Math.atan2(dx, dz), 0); sampler.press('KeyW'); requestAnimationFrame(next); }
        };
        next();
      });
    },
    throwFrag: () => { pulseGrenade = true; },
    press: (key: string) => sampler.press(key),
    release: (key: string) => sampler.release(key),
  };
}

declare global {
  interface Window {
    joinSuppressLoop: typeof join;
    suppressLoop: Awaited<ReturnType<typeof join>>;
  }
}
window.joinSuppressLoop = join;
