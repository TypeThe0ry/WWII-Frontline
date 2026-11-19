import { expect, test } from '@playwright/test';
import { MOTION, RELOAD_BIT } from '../src/render/actors/anim-graph';

test.use({ viewport: { width: 752, height: 437 } });

for (const backend of ['webgl2', 'webgpu'] as const) {
  test(`60 placeholder actors run the animation graph on ${backend}`, async ({ page }, info) => {
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()); });
    // WebGPU on SwiftShader draws offscreen; only WebGL2 produces a visible screenshot in this setup.
    if (backend === 'webgpu') await page.addInitScript({ path: 'e2e/webgpu-swiftshader-shim.js' });
    await page.goto(`/e2e/actors-fixture.html?renderer=${backend}`);
    await page.waitForFunction(() => !!window.actorFixture);
    const sample = await page.evaluate(({ walk, reload }) => {
      const { backend, state, bus, manager, frame, stats } = window.actorFixture!;
      const actor = manager.actor(10)!;
      const other = manager.actor(11)!;
      const startHeight = actor.height;
      const e = state.entities;
      e.stance[0] = 1;
      e.anim[0] = walk | reload;
      e.pos[0]! += 0.24;
      frame(125);
      const halfway = actor.height;
      const gait = actor.skeleton.joints.LeftUpperLeg.rotation.x - actor.skeleton.joints.RightUpperLeg.rotation.x;
      const duringReload = actor.animation.upper;
      frame(125);
      const crouch = actor.height;
      const reloadMuzzleY = actor.weapon!.muzzle.getAbsolutePosition().y;
      const reloadElbow = actor.skeleton.joints.RightLowerArm.rotation.x;
      e.anim[0] = 0;
      frame(150);
      const afterReload = actor.animation.upper;
      const aimMuzzleY = actor.weapon!.muzzle.getAbsolutePosition().y;
      bus.emit('fire.remote', { shooter: 10, weapon: 0, muzzlePos: { x: 0, y: 1, z: 1 }, dir: { x: 0, y: 0, z: 1 }, distance: 10 });
      frame(16);
      const shot = { upper: actor.animation.upper, pitch: actor.aim.rotation.x, otherPitch: other.aim.rotation.x };
      bus.emit('death', { victim: 10, respawnAfter: 0, canBeRevived: true });
      frame(16);
      const downed = actor.animation.life;
      bus.emit('revive', { medic: 11, target: 10 });
      frame(16);
      const revived = actor.animation.life;
      e.health[0] = 0;
      frame(16);
      const dead = actor.animation.life;
      const { activeMeshes, triangles } = stats();
      return { backend, count: manager.actorCount, activeMeshes, triangles, startHeight, halfway, crouch, gait,
        aimMuzzleY, reloadMuzzleY, reloadElbow, duringReload, afterReload, shot, downed, revived, dead };
    }, { walk: MOTION.walk, reload: RELOAD_BIT });
    expect(sample.backend).toBe(backend);
    expect(sample.count).toBe(60);
    expect(sample.activeMeshes).toBeGreaterThan(60);
    expect(sample.triangles).toBeGreaterThan(0);
    expect(sample.halfway).toBeCloseTo((sample.startHeight + sample.crouch) / 2);
    expect(sample.crouch).toBeLessThan(sample.startHeight);
    expect(Math.abs(sample.gait)).toBeGreaterThan(0.01);
    expect(sample.duringReload).toBe('reload');
    expect(sample.reloadElbow).toBeGreaterThan(0);
    expect(sample.reloadMuzzleY).toBeLessThan(sample.aimMuzzleY);
    expect(sample.afterReload).toBe('aim');
    expect(sample.shot.upper).toBe('fire');
    expect(sample.shot.pitch).toBeLessThan(-0.04);
    expect(sample.shot.otherPitch).toBeCloseTo(0);
    expect([sample.downed, sample.revived, sample.dead]).toEqual(['downed', 'alive', 'dead']);
    if (backend === 'webgl2') await info.attach('fixed-camera placeholder frame', { body: await page.screenshot(), contentType: 'image/png' });
    expect(errors).toEqual([]);
  });
}
