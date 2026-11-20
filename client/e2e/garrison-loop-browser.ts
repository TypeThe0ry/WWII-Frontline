import { createDeployActions, type DeployRequest } from '../src/game/deploy-actions';
import { EventBus } from '../src/game/event-bus';
import { FixedStepper } from '../src/game/fixed-step';
import { GatewayClient } from '../src/game/gateway-client';
import { startNetGame } from '../src/game/net-game';
import type { PlacementKind } from '../src/game/placement';
import { connect } from '../src/net/connect';
import { parseCertHash, parseNetConfig } from '../src/net/config';
import type { TransportKind } from '../src/net/transport';

async function admission(nick: string, transport: TransportKind, certHash: string) {
  const gateway = new GatewayClient('');
  const [match] = await gateway.list();
  if (!match?.compatible || match.map !== 'carentan/m1') throw new Error('fixture match not listed');
  const guest = await gateway.guest();
  const join = await gateway.join(match.id, guest, nick);
  return {
    config: { host: join.host, wsPort: join.wsPort, wtPort: join.wtPort, secure: false, transport,
      ...(certHash ? { certHash: parseCertHash(certHash) } : {}) },
    ticket: new TextEncoder().encode(join.ticket),
  };
}

async function join(nick: string, transport: TransportKind, hash: string) {
  const bus = new EventBus();
  const net = await startNetGame(document.querySelector('canvas')!, bus, '/', undefined, await admission(nick, transport, hash));
  const { sampler, session, gameState, game, wasm } = net;
  const actions = createDeployActions(session.transport, wasm);
  const decoder = new wasm.SnapshotDecoder();
  const peers = new Map<number, { x: number; z: number; health: number; tick: number }>();
  let lastTick = -1;
  const offSnap = session.onSnapshot((bytes) => {
    const tick = decoder.decode(bytes);
    if (tick <= lastTick) return;
    lastTick = tick;
    const ids = decoder.ids(), pos = decoder.pos(), health = decoder.health();
    for (let i = 0; i < ids.length; i++) {
      peers.set(ids[i]!, { x: pos[i * 3]!, z: pos[i * 3 + 2]!, health: health[i]!, tick });
    }
  });
  const spawnEvents: { id: number; state: number; at: number }[] = [];
  bus.on('spawnpoint.changed', ({ id, state }) => spawnEvents.push({ id, state, at: performance.now() }));
  const stepper = new FixedStepper();
  let last = performance.now();
  const timer = setInterval(() => {
    const now = performance.now();
    const dt = now - last;
    last = now;
    const { steps, alpha } = stepper.advance(dt);
    net.update(steps, alpha, dt);
  }, 1000 / 60);

  const reserves: Awaited<ReturnType<typeof connect>>[] = [];
  const dispose = () => {
    clearInterval(timer);
    for (const reserve of reserves) reserve.close();
    actions.dispose();
    offSnap();
    decoder.free();
    net.dispose();
  };
  window.addEventListener('pagehide', dispose, { once: true });

  // Undeployed sessions satisfy the real team-balancer and the Support squad-size requirement.
  async function addReserve(nick: string, team: number, squad?: number) {
    const standby = await admission(nick, 'ws', '');
    const config = parseNetConfig('?server=same', location);
    const reserve = await connect({ wasm, config, hello: { ticket: standby.ticket } });
    reserves.push(reserve);
    if (reserve.welcome.team !== team) throw new Error('unexpected reserve assignment');
    if (squad !== undefined) {
      const reserveActions = createDeployActions(reserve.transport, wasm);
      try { reserveActions.joinSquad(squad); } finally { reserveActions.dispose(); }
    }
    return reserve.welcome.playerId;
  }

  const status = () => {
    const { net: state, local } = game.state;
    return { health: state.health.peek(), ammo: state.ammoAuth.peek(), reserve: state.reserveAuth.peek(),
      tick: state.snapshotTick.peek(), pos: Array.from(local.pos) };
  };

  function place(kind: PlacementKind, commit = true) {
    const slot = { garrison: 'Digit6', supply_crate: 'Digit7', ammo_crate: 'Digit8' }[kind];
    sampler.setView(0, -1.39); // short terrain ray, within range of the stationary builder
    const start = performance.now();
    sampler.press(slot);
    sampler.release(slot);
    return new Promise<{ cost?: number; reason?: string; selectToAckMs?: number; confirmToAckMs?: number }>((resolve, reject) => {
      let confirmedAt = 0;
      let cost: number | undefined;
      const off = net.placement.view.subscribe(({ state, preview }) => {
        if (state.mode === 'idle' && confirmedAt) {
          const now = performance.now();
          finish();
          resolve({ cost, selectToAckMs: now - start, confirmToAckMs: now - confirmedAt });
        } else if (preview?.reasonKey && preview.reasonKey !== 'ui.officer.err.previewUnavailable') {
          finish();
          resolve({ reason: preview.reasonKey });
        } else if (preview?.legal && !confirmedAt) {
          cost = preview.supplyCost;
          if (!commit) {
            finish();
            resolve({ cost });
          } else {
            confirmedAt = performance.now();
            sampler.press('Mouse0');
            sampler.release('Mouse0');
          }
        }
      });
      const deadline = setTimeout(() => {
        const mode = net.placement.view.peek().state.mode;
        finish();
        reject(new Error(`${kind} placement timed out (${mode})`));
      }, 5000);
      function finish() {
        clearTimeout(deadline);
        off();
        sampler.releaseAll();
      }
    });
  }

  return {
    id: session.welcome.playerId, team: session.welcome.team, transport: session.kind,
    status, spawnEvents, addReserve,
    peers: (id: number) => peers.get(id), roster: () => gameState.roster.list(),
    points: () => [...gameState.spawnPoints.values()],
    createSquad: () => actions.createSquad('infantry', 'Alpha'),
    joinSquad: (id: number) => actions.joinSquad(id),
    deploy: (req: DeployRequest) => actions.deploy(req),
    place,
    hold: (key: string) => sampler.press(key), release: (key: string) => sampler.release(key),
    move: (x: number, z: number, sprint: boolean) => {
      const [px, , pz] = status().pos;
      sampler.setView(Math.atan2(x - px!, z - pz!), 0);
      sampler.press('KeyW');
      if (sprint) sampler.press('ShiftLeft');
    },
    stopMove: () => { sampler.release('ShiftLeft'); sampler.release('KeyW'); },
    stop: () => sampler.releaseAll(),
  };
}

declare global {
  interface Window {
    joinGarrisonFixture: typeof join;
    garrisonFixture: Awaited<ReturnType<typeof join>>;
  }
}
window.joinGarrisonFixture = join;
