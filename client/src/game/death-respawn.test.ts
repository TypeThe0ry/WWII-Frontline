// M0-030 (PI-028 handover): the browser client freezes its prediction while the server says the player is dead and
// replays a respawn teleport from a fresh life, like crates/headless-client/src/predictor.rs.
import { describe, expect, it } from 'vitest';
import { CAN_RUN, HAVE_TERRAIN, HAVE_WASM, REQUIRE, runScenario, walkCircle, type Scenario } from './test-harness';

if (!CAN_RUN && REQUIRE) throw new Error(`FRONTLINE_REQUIRE_TERRAIN=1 but wasm pkg (${HAVE_WASM}) or terrain chunks (${HAVE_TERRAIN}) is missing`);

const link = (oneWayMs: number, jitterMs = 0, loss = 0) => ({ oneWayMs, jitterMs, loss });
const DEATH_S = 20;
const RESPAWN_S = 5;

async function run(over: Partial<Scenario> = {}, extra?: Scenario['onFrame']) {
  const dead: { moved: number; fires: number; frames: number } = { moved: 0, fires: 0, frames: 0 };
  let prev: [number, number] | undefined;
  const r = await runScenario({
    seconds: 40,
    up: link(50),
    down: link(50),
    script: (n) => ({ ...walkCircle(n), buttons: n % 15 === 0 ? 1 : 0 }), // walks and keeps pulling the trigger
    seed: 11,
    death: { atS: DEATH_S, respawnAfterS: RESPAWN_S },
    onFrame: (g, _f, _ms, t) => {
      const s = t / 1000;
      const [x, z] = [g.state.local.simPos[0] as number, g.state.local.simPos[2] as number];
      // 1 s after the death snapshot until the respawn: nothing may move
      if (s > DEATH_S + 1 && s < DEATH_S + RESPAWN_S - 0.2 && prev) {
        dead.frames++;
        dead.moved += Math.hypot(x - prev[0], z - prev[1]);
      }
      prev = [x, z];
      extra?.(g, _f, _ms, t);
    },
    ...over,
  });
  return { r, dead };
}

describe.skipIf(!CAN_RUN)('death and respawn', () => {
  it('prediction is frozen while dead and a respawn teleport is a clean replay from a fresh life', async () => {
    let fireWhileDead = 0;
    let dyingPos: [number, number] | undefined;
    let respawnErr = Infinity;
    let ammoAfter = -1;
    let nowS = 0;
    const { r, dead } = await run(
      {
        setup: (_g, bus) => {
          bus.on('fire.local', () => {
            if (nowS > DEATH_S + 1 && nowS < DEATH_S + RESPAWN_S - 0.2) fireWhileDead++;
          });
        },
      },
      (g, _f, _ms, t) => {
        const s = (nowS = t / 1000);
        if (s > DEATH_S + 2 && s < DEATH_S + 2.1 && !dyingPos) dyingPos = [g.state.local.simPos[0] as number, g.state.local.simPos[2] as number];
        if (s > DEATH_S + RESPAWN_S + 3 && s < DEATH_S + RESPAWN_S + 3.1) {
          respawnErr = Math.hypot((g.state.local.simPos[0] as number) - 128, (g.state.local.simPos[2] as number) - 56);
          ammoAfter = g.state.local.ammo;
        }
      },
    );
    console.log(`[death] ${JSON.stringify({ times: r.correctionTimesS.map((x) => +x.toFixed(2)), corrections: r.corrections, snaps: r.snaps, movedWhileDead: +dead.moved.toFixed(3), deadFrames: dead.frames, fireWhileDead, respawnErrM: +respawnErr.toFixed(3), ammoAfter, respawned: r.respawned, replayed: r.replayedCommands })}`);
    expect(r.respawned).toBe(true);
    expect(dyingPos).toBeDefined();
    expect(dead.frames).toBeGreaterThan(100);
    expect(dead.moved).toBeLessThan(0.05); // frozen: zero movement while dead
    expect(fireWhileDead).toBe(0);
    // Corrections: the start-up sync, the death (the client ran on for the commands in flight: one snap), the respawn
    // teleport (one snap) and the few that follow while the acknowledgements of the commands sent while dead catch up.
    // Never while dead and never later than 2 s after the respawn (before the fix: 53 corrections, 33 m of walking dead).
    const at = r.correctionTimesS.filter((t) => t > 1);
    expect(at.filter((t) => t > DEATH_S + 1 && t < DEATH_S + RESPAWN_S)).toEqual([]);
    expect(at.filter((t) => t > DEATH_S + RESPAWN_S + 2)).toEqual([]);
    expect(r.snaps).toBeLessThanOrEqual(2);
    expect(r.corrections).toBeLessThanOrEqual(7);
    // 3 s after respawning the player is within 3 s of walking from the spawn point (13.5 m at ~4.4 m/s: it did not stay
    // where it died, ~60 m away) and has a fresh loadout (ammo in the magazine)
    expect(respawnErr).toBeLessThan(20);
    expect(ammoAfter).toBeGreaterThan(0);
  }, 120_000);

  it('a runner that dies and respawns settles within 2 s and ends where the server is', async () => {
    const { r } = await run({ script: walkCircle });
    const g = r.game.state.local;
    const err = Math.hypot((g.simPos[0] as number) - r.serverPos[0], (g.simPos[2] as number) - r.serverPos[2]);
    console.log(`[death] runner ${JSON.stringify({ corrections: r.corrections, snaps: r.snaps, endErrM: +err.toFixed(3) })}`);
    expect(r.respawned).toBe(true);
    expect(r.snaps).toBeLessThanOrEqual(2);
    expect(r.corrections).toBeLessThanOrEqual(7);
    expect(r.correctionTimesS.filter((t) => t > DEATH_S + RESPAWN_S + 2)).toEqual([]);
    // the client is ahead of the server by the commands in flight (RTT 100 ms at ~4.4 m/s), never by more
    expect(err).toBeLessThan(1.5);
  }, 120_000);
});
