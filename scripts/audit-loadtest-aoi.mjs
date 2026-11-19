const CLOSE_M = 105.6;
const FAR_M = 440;
const RAD_TO_DEG = 180 / Math.PI;
const CONE_DEG = 110;
const QPOS_M = 0.0271; // sqrt(3) * half a 1/32 m step, rounded up
const EPSILON = 1e-6;
const DECIMAL = /^[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:e[+-]?[0-9]+)?$/i;

function parseTrace(tag, body, offset, size, stride = 0) {
  const tokens = body.trim().split(/\s+/);
  const fail = (detail) => { throw new Error(`Invalid [aoi-${tag}] at offset ${offset}: ${detail}`); };
  if (!/^[ \t]/.test(body) || tokens.length < size ||
      (stride ? (tokens.length - size) % stride !== 0 : tokens.length !== size)) {
    fail('wrong field count or separator');
  }
  const uint = (i, max) => {
    const n = Number(tokens[i]);
    if (!/^[0-9]+$/.test(tokens[i]) || n > max) fail(`invalid integer field ${i + 1}`);
    return n;
  };
  const floats = (start, end) => tokens.slice(start, end).map((token, i) => {
    const n = Number(token);
    if (!DECIMAL.test(token) || !Number.isFinite(Math.fround(n))) fail(`invalid numeric field ${start + i + 1}`);
    return n;
  });
  const player = uint(0, 255);
  const tick = uint(1, 0xffffffff);
  return { player, tick, key: `${player}:${tick}`, length: tokens.length, uint, floats, fail };
}

function directions(angles) {
  const view = [];
  for (let i = 0; i < angles.length; i += 2) {
    const yaw = angles[i];
    const pitch = angles[i + 1];
    const cp = Math.cos(pitch);
    view.push([Math.sin(yaw) * cp, Math.sin(pitch), Math.cos(yaw) * cp]);
  }
  return view;
}

function compare(target, origin, view, legacy = false) {
  const dx = target[0] - origin[0];
  const dy = target[1] - origin[1];
  const dz = target[2] - origin[2];
  const distance_m = Math.hypot(dx, dy, dz);
  const chestDy = dy + 1.0 - 1.6;
  const length = Math.hypot(dx, chestDy, dz);
  let best_angle_deg = null;
  if (length > 0 && view.length > 0) {
    let bestCos = -1;
    for (const [x, y, z] of view) {
      bestCos = Math.max(bestCos, (x * dx + y * chestDy + z * dz) / length);
    }
    best_angle_deg = Math.acos(Math.min(1, bestCos)) * RAD_TO_DEG;
  }
  const posTol = legacy ? 4 : QPOS_M;
  const coneTol = legacy ? 5 : Math.asin(Math.min(1, QPOS_M / length)) * RAD_TO_DEG;
  const allowed = distance_m <= CLOSE_M + posTol + EPSILON ||
    (distance_m <= FAR_M + posTol + EPSILON && best_angle_deg !== null &&
      best_angle_deg <= CONE_DEG + coneTol + EPSILON);
  return { distance_m, best_angle_deg, allowed };
}

/** Audit every decoded snapshot against the observer pose at the same player/tick. */
export function auditAoi(serverLog, headlessLog) {
  const poses = new Map();
  for (const { 1: body, index } of serverLog.matchAll(/^\[aoi-pose\]([^\r\n]*)/gm)) {
    const trace = parseTrace('pose', body, index, 10);
    const pose = [trace.uint(2, 1), ...trace.floats(3, 10)];
    const old = poses.get(trace.key);
    if (old && old.some((value, i) => value !== pose[i])) {
      trace.fail(`conflicting duplicate pose for player ${trace.player}, tick ${trace.tick}`);
    }
    poses.set(trace.key, pose);
  }

  // Keep wire records only for snapshot keys with legacy candidates. A candidate
  // uses the latest preceding wire line for its key, even with interleaved logs.
  const candidateWires = new Map();
  for (const { 1: body, index } of headlessLog.matchAll(/^\[aoi-pred\]([^\r\n]*)/gm)) {
    const trace = parseTrace('pred', body, index, 6, 2);
    candidateWires.set(trace.key, null);
  }

  const result = {
    snapshots: 0, matched_snapshots: 0, missing_pose_snapshots: 0,
    records_checked: 0, beyond_close_records: 0, leaks: 0, worst_leak_m: 0,
    violations: [], legacy_candidates: [],
  };
  for (const { 1: tag, 2: body, index } of headlessLog.matchAll(/^\[aoi-(wire|pred)\]([^\r\n]*)/gm)) {
    const trace = tag === 'wire' ? parseTrace(tag, body, index, 2, 5) : parseTrace(tag, body, index, 6, 2);
    const { player, tick, key } = trace;
    const pose = poses.get(key);
    const origin = pose?.slice(1, 4);
    const view = pose && directions(pose.slice(4));

    if (tag === 'wire') {
      result.snapshots++;
      if (pose) result.matched_snapshots++;
      else result.missing_pose_snapshots++;

      const records = new Map();
      for (let i = 2; i < trace.length; i += 5) {
        const enemy_id = trace.uint(i, 255);
        const enemy_faction = trace.uint(i + 1, 1);
        const enemy_pos = trace.floats(i + 2, i + 5);
        if (records.has(enemy_id)) trace.fail(`duplicate Player record ${enemy_id}`);
        const record = { player, tick, enemy_id, enemy_faction, enemy_pos };
        records.set(enemy_id, record);
        if (!pose || enemy_faction === pose[0]) continue;

        const geometry = compare(enemy_pos, origin, view);
        result.records_checked++;
        if (geometry.distance_m > CLOSE_M) result.beyond_close_records++;
        if (!geometry.allowed) {
          result.violations.push({ ...record, ...geometry });
          result.worst_leak_m = Math.max(result.worst_leak_m, geometry.distance_m);
        }
      }
      if (candidateWires.has(key)) candidateWires.set(key, records);
    } else {
      const enemyId = trace.uint(2, 255);
      const record = candidateWires.get(key)?.get(enemyId);
      if (!record) trace.fail(`missing legacy record ${enemyId} in preceding wire for player ${player}, tick ${tick}`);
      if (pose && record.enemy_faction === pose[0]) trace.fail(`legacy candidate ${enemyId} is an ally`);
      const predicted = trace.floats(3, 6);
      const history = directions(trace.floats(6));
      const target = record.enemy_pos;
      result.legacy_candidates.push({
        ...record,
        authoritative: pose ? compare(target, origin, view) : null,
        legacy: compare(target, predicted, history, true),
        auth_pos_legacy_view: pose ? compare(target, origin, history, true) : null,
        legacy_pos_auth_view: pose ? compare(target, predicted, view, true) : null,
      });
    }
  }
  result.leaks = result.violations.length;
  return result;
}
