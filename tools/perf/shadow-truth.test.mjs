import assert from 'node:assert/strict';
import test from 'node:test';
import { inShadow, pointInPoly } from './shadow-truth.mjs';

const prism = { ring: [[0, 0], [10, 0], [10, 10], [0, 10]], minX: 0, maxX: 10, minZ: 0, maxZ: 10, base: 0, top: 6 };

test('pointInPoly', () => {
  assert.equal(pointInPoly(5, 5, prism.ring), true);
  assert.equal(pointInPoly(-1, 5, prism.ring), false);
});

test('a ground point beside a 6 m building is shadowed only on the side away from the sun', () => {
  const sun = (() => { const v = [-0.45, 0.8, -0.35]; const l = Math.hypot(...v); return v.map((x) => x / l); })(); // towards the sun
  // sun towards -x, -z: the shadow falls to +x, +z of the building. 3 m east of the east wall is shadowed, 3 m west of the west wall is not
  assert.equal(inShadow(13, 0.05, 5, sun, [prism]), true);
  assert.equal(inShadow(-3, 0.05, 5, sun, [prism]), false);
  // far enough away that the 6 m building's shadow (~ 6 * 0.45 / 0.8 = 3.4 m) cannot reach
  assert.equal(inShadow(20, 0.05, 5, sun, [prism]), false);
});
