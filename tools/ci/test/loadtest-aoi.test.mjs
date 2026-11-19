import assert from 'node:assert/strict';
import { test } from 'node:test';
import { auditAoi } from '../../../scripts/audit-loadtest-aoi.mjs';

const pose = '[aoi-pose] 1 42 0 0 0 0 0 0 0 0';
const wire = (records) => `[aoi-wire] 1 42 ${records}`;

test('detects range and cone leaks even without a legacy candidate', () => {
  const audit = auditAoi(pose, wire('2 1 0 0 441  3 1 0 0 -200  4 0 0 0 900  5 1 0 0 -100'));
  assert.equal(audit.records_checked, 3);
  assert.equal(audit.beyond_close_records, 2);
  assert.equal(audit.leaks, 2);
  assert.equal(audit.worst_leak_m, 441);
  assert.deepEqual(audit.violations.map((v) => [v.player, v.tick, v.enemy_id, v.distance_m]), [
    [1, 42, 2, 441], [1, 42, 3, 200],
  ]);
  assert.ok(audit.violations[0].best_angle_deg < 1);
  assert.ok(audit.violations[1].best_angle_deg > 179);
  assert.deepEqual(audit.legacy_candidates, []);
});

test('compares the legacy pose and both mixed poses for each flagged record', () => {
  const audit = auditAoi(pose, [
    wire('2 1 0 0 437  3 1 0 0 200'),
    '[aoi-wire] 2 42',
    '[aoi-pred] 1 42 2 0 0 -9 0 0',
    `[aoi-pred] 1 42 3 0 0 0 ${2 * Math.PI / 3} 0 ${Math.PI} 0`,
  ].join('\n'));
  const [position, orientation] = audit.legacy_candidates;
  assert.equal(audit.leaks, 0);
  assert.equal(audit.legacy_candidates.length, 2);
  assert.equal(position.authoritative.distance_m, 437);
  assert.equal(position.legacy.distance_m, 446);
  assert.ok(position.authoritative.allowed && !position.legacy.allowed);
  assert.ok(position.auth_pos_legacy_view.allowed && !position.legacy_pos_auth_view.allowed);
  assert.ok(orientation.authoritative.best_angle_deg < 1);
  assert.ok(Math.abs(orientation.legacy.best_angle_deg - 120) < 0.01);
  assert.equal(orientation.legacy.allowed, false);
  assert.ok(!orientation.auth_pos_legacy_view.allowed && orientation.legacy_pos_auth_view.allowed);
});

test('counts empty, stale, and duplicate decodes by player and tick', () => {
  const stale = wire('2 1 0 0 100');
  const audit = auditAoi(`${pose}\n[aoi-pose] 1 43 0 0 0 0 0 0 0 0\n${pose}\n`, [
    'unrelated [aoi-wire] 1 42',
    '[aoi-wire] 1 43',
    stale,
    stale,
    '[aoi-wire] 2 42 1 0 0 0 500',
    '[aoi-pred] 2 42 1 0 0 0 0 0',
  ].join('\r\n'));
  assert.deepEqual([audit.snapshots, audit.matched_snapshots, audit.missing_pose_snapshots], [4, 3, 1]);
  assert.equal(audit.records_checked, 2);
  assert.equal(audit.worst_leak_m, 0);
  assert.equal(audit.legacy_candidates[0].authoritative, null);
  assert.equal(audit.legacy_candidates[0].legacy.distance_m, 500);
});

test('includes both views and uses pitch and three-dimensional distance', () => {
  const pitches = `[aoi-pose] 1 42 0 0 160 0 0 ${Math.PI / 3} 0 ${-Math.PI / 3}`;
  const audit = auditAoi(pitches, wire('2 1 0 310 -100  3 1 0 10 -100  4 1 0 160 -150'));
  assert.equal(audit.beyond_close_records, 3);
  assert.deepEqual(audit.violations.map((v) => v.enemy_id), [4]);
});

test('applies only the QPos distance and angle envelopes, with a nominal close count', () => {
  const bearing = (degrees) => {
    const angle = degrees * Math.PI / 180;
    return `${200 * Math.sin(angle)} 0.6 ${200 * Math.cos(angle)}`;
  };
  const audit = auditAoi(pose, wire([
    '2 1 0 0 -105.62', '3 1 0 0 -105.64',
    '4 1 0 0 440.02', '5 1 0 0 440.04',
    `6 1 ${bearing(110.005)}`, `7 1 ${bearing(110.010)}`,
  ].join(' ')));
  assert.equal(audit.beyond_close_records, 6);
  assert.deepEqual(audit.violations.map((v) => v.enemy_id), [3, 5, 7]);
});

test('rejects malformed traces, conflicting poses, and missing legacy records', () => {
  assert.throws(() => auditAoi(`${pose}\n${pose.replace('42 0', '42 1')}`, ''), /conflicting duplicate pose/);
  assert.throws(() => auditAoi(pose, wire('2 1 0 0')), /Invalid \[aoi-wire\]/);
  assert.throws(() => auditAoi(pose, wire('2 0 0 NaN 0')), /invalid numeric/);
  assert.throws(() => auditAoi(pose, wire('2 2 0 0 0')), /invalid integer/);
  const pred = '[aoi-pred] 1 42 2 0 0 0 0 0';
  assert.throws(() => auditAoi(pose, `${wire('3 1 0 0 0')}\n${pred}`), /missing legacy record/);
  assert.throws(() => auditAoi(pose, `${pred}\n${wire('2 1 0 0 0')}`), /missing legacy record/);
  assert.throws(() => auditAoi(pose, `${wire('2 1 0 0 0')}\n${pred} 0`), /Invalid \[aoi-pred\]/);
});
