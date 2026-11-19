import { signal } from '@preact/signals';
import { render } from 'preact';
import { EventBus, type GameEvents } from '../src/game';
import { createDeployActions, HQ_SPAWN_ID } from '../src/game/deploy-actions';
import { loadWasm } from '../src/game/net-game';
import { PLACE_ERROR_KEY, placementNext } from '../src/game/placement';
import { PlacementActions } from '../src/game/placement-actions';
import { GameState } from '../src/game/state';
import { Handlers } from '../src/net/transport';
import { DownedDeath, createDownedUi } from '../src/ui/downed';
import { Deploy } from '../src/ui/deploy';
import { Hud, createHudStore } from '../src/ui/hud';
import '../src/ui/i18n/catalogs';
import type { MapSpawn } from '../src/ui/map';
import { PlacementGhostLabel, SquadToolPanel, VehicleEnterPrompt } from '../src/ui/placement';

const points: MapSpawn[] = [
  { id: HQ_SPAWN_ID, kind: 'hq', x: 1024, z: 1024, status: 'available', label: 'Friendly HQ' },
  { id: 4, kind: 'garrison', x: 1400, z: 800, status: 'available', label: 'North garrison' },
];
const wasm = await loadWasm(import.meta.env.BASE_URL);
const c0 = new Handlers<[Uint8Array]>();
const c0Sent = signal<number[][]>([]);
const nowTick = signal(130);
let reject = true;
const commands = createDeployActions({
  onC0: (cb) => c0.add(cb), onClose: () => () => {},
  sendC0(bytes) {
    c0Sent.value = [...c0Sent.peek(), [...bytes]];
    if (bytes[0] !== 0x10) return;
    // Fixture (not a live server): C0 DeployResult [ok:1][tick:32][reason:8][pad:7], MSB first.
    const result = reject ? Uint8Array.of(0x11, 0, 0, 0, 0, 10, 0x80) : Uint8Array.of(0x11, 0x80, 0, 0, 0x50, 0, 0);
    reject = false;
    setTimeout(() => c0.emit(result), 80);
  },
}, wasm);

const bus = new EventBus<GameEvents>();
const game = new GameState();
const connected = signal(true);
const hp = signal(40);
const respawnClock = signal({ spawnTick: 160, nowTick: 130, tickRate: 30 });
const calls = signal(0);
let now = 10_000;
const downed = createDownedUi({ inputs: { connected, downedHp: hp, respawnClock }, actions: { callMedic: () => { calls.value++; return true; } }, now: () => now });
const hud = createHudStore({ game, bus, selfId: 1, inputs: { phase: downed.phase }, now: () => now });

const placementIntents = signal(0);
const placement = new PlacementActions(
  (state, point) => placementNext(state, { t: 'check', result: point[0] < 0
    ? { legal: false, reasonKey: PLACE_ERROR_KEY.Slope }
    : { legal: true, supplyCost: 50 } }),
  async () => {
    placementIntents.value++;
    return { ok: false, reasonKey: PLACE_ERROR_KEY.NoSupplies };
  },
);

function PlacementFixture() {
  const current = placement.view.value;
  return <section aria-label="Placement fixture">
    <SquadToolPanel tools={[{ kind: 'garrison', label: 'Garrison kit', enabled: true }]}
      activeTool={current.state.kind} onSelectTool={(kind) => { placement.enter(kind); placement.update([-1, 0, 0]); }} />
    <button onClick={() => placement.update([1, 0, 0])}>Aim at valid ground</button>
    <button disabled={!current.preview?.legal || current.state.mode !== 'placing'} onClick={() => void placement.commit()}>Place Garrison</button>
    <output aria-label="Placement intents">{placementIntents.value}</output>
    {current.preview && <PlacementGhostLabel preview={current.preview} pending={current.state.mode === 'committing'} />}
    <VehicleEnterPrompt vehicleId={128} name="Fixture vehicle" canBoardNearest={false}
      seats={[{ seat: 'driver', occupied: false }, { seat: 'gunner', occupied: true }]} />
  </section>;
}

function Fixture() {
  return <main>
    <Deploy actions={commands} spawnPoints={points} squadId={2} roles={[{ index: 3, id: 'rifleman', label: 'Rifleman' }]}
      loadouts={[{ index: 12, roleId: 'rifleman', label: 'Standard issue' }]} nowTick={nowTick.value} tickHz={30} />
    <output aria-label="C0 packets sent">{JSON.stringify(c0Sent.value)}</output>
    <button onClick={() => { nowTick.value = 160; }}>Advance spawn clock</button>
    <button onClick={() => {
      game.net.health.value = 63; game.net.stamina.value = 32; game.net.ammoAuth.value = 3; game.net.snapshotTick.value = 45;
      game.local.yaw = Math.PI / 2; now += 100;
      bus.emit('snapshot.applied', { tick: 45, entityCount: 100 });
      bus.emit('hit.confirm', { target: 2, zone: 0, damage: 25, killed: false });
      hud.update();
    }}>Apply fixture snapshot</button>
    <button onClick={() => { downed.transition('down'); downed.setKiller({ name: 'Fixture Opponent', weapon: 2, distM: 40, isBot: false, friendly: false, hidden: false }); }}>Down fixture player</button>
    <button onClick={() => { downed.transition('die'); downed.setKiller({ name: 'Fixture Opponent', weapon: 2, distM: 40, isBot: false, friendly: false, hidden: false }); }}>Kill fixture player</button>
    <button onClick={() => downed.transition('respawn')}>Respawn fixture player</button>
    <output aria-label="Medic call intents">{calls.value}</output>
    <Hud store={hud} />
    <DownedDeath store={downed} />
    <PlacementFixture />
  </main>;
}
render(<Fixture />, document.querySelector('#fixture')!);
