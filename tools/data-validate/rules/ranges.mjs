// Value-range rules for numeric fields (ARCHITECTURE 8.5, task M0-004).

const num = (v) => typeof v === 'number' && Number.isFinite(v);

function mvCheck(errors, value, pointer) {
  if (num(value) && (value < 50 || value > 1500)) {
    errors.push({ pointer, message: `muzzle_velocity_mps out of range [50,1500]: ${value}` });
  }
}

function checkWeapon(doc, errors) {
  const g = doc.gameplay;
  if (doc.historical) mvCheck(errors, doc.historical.muzzle_velocity_mps, '/historical/muzzle_velocity_mps');
  if (!g || typeof g !== 'object') return;
  if (g.ballistics) mvCheck(errors, g.ballistics.muzzle_velocity_mps, '/gameplay/ballistics/muzzle_velocity_mps');

  const f = g.fire;
  if (f && typeof f === 'object') {
    if (num(f.cap_rpm) && (f.cap_rpm < 1 || f.cap_rpm > 2000)) {
      errors.push({ pointer: '/gameplay/fire/cap_rpm', message: `cap_rpm out of range [1,2000]: ${f.cap_rpm}` });
    }
    const cycled = f.mode === 'bolt' || f.mode === 'pump';
    if (cycled) {
      if (f.bolt_cycle_ms === undefined) {
        errors.push({ pointer: '/gameplay/fire', message: `${f.mode} fire mode requires bolt_cycle_ms` });
      }
      if (f.cap_rpm !== undefined) {
        errors.push({ pointer: '/gameplay/fire/cap_rpm', message: `${f.mode} fire mode must not set cap_rpm` });
      }
    } else if (f.mode !== undefined) {
      if (f.cap_rpm === undefined) {
        errors.push({ pointer: '/gameplay/fire', message: `${f.mode} fire mode requires cap_rpm` });
      }
      if (f.bolt_cycle_ms !== undefined) {
        errors.push({ pointer: '/gameplay/fire/bolt_cycle_ms', message: `${f.mode} fire mode must not set bolt_cycle_ms` });
      }
    }
  }

  if (Array.isArray(g.falloff)) {
    let prev = -Infinity;
    g.falloff.forEach((p, i) => {
      if (!p || typeof p !== 'object') return;
      if (num(p.dist_m)) {
        if (p.dist_m <= prev) {
          errors.push({
            pointer: `/gameplay/falloff/${i}/dist_m`,
            message: `falloff dist_m must be strictly increasing (${p.dist_m} <= ${prev})`,
          });
        }
        prev = p.dist_m;
      }
      if (num(p.mult) && !(p.mult > 0 && p.mult <= 1)) {
        errors.push({ pointer: `/gameplay/falloff/${i}/mult`, message: `falloff mult must be in (0,1]: ${p.mult}` });
      }
    });
  }

  if (g.part_mult && typeof g.part_mult === 'object') {
    for (const [k, v] of Object.entries(g.part_mult)) {
      if (num(v) && !(v > 0 && v <= 10)) {
        errors.push({ pointer: `/gameplay/part_mult/${k}`, message: `part_mult.${k} must be in (0,10]: ${v}` });
      }
    }
  }
}

function checkRole(doc, errors) {
  const m = doc.movement;
  if (!m || typeof m !== 'object') return;
  for (const k of ['prone_mps', 'crouch_walk_mps', 'crouch_mps', 'walk_mps', 'run_mps', 'sprint_mps']) {
    if (num(m[k]) && (m[k] <= 0 || m[k] > 15)) {
      errors.push({ pointer: `/movement/${k}`, message: `${k} out of range (0,15]: ${m[k]}` });
    }
  }
  // GAMEDESIGN 9.1: crouch run (2.4) is faster than standing walk (2.0), so the order is a
  // partial one: prone < crouch_walk < crouch < run < sprint and prone < walk < run.
  const pairs = [
    ['prone_mps', 'crouch_walk_mps'],
    ['crouch_walk_mps', 'crouch_mps'],
    ['prone_mps', 'crouch_mps'],
    ['crouch_mps', 'run_mps'],
    ['prone_mps', 'walk_mps'],
    ['walk_mps', 'run_mps'],
    ['run_mps', 'sprint_mps'],
  ];
  for (const [lo, hi] of pairs) {
    const a = m[lo];
    const b = m[hi];
    if (num(a) && num(b) && !(a < b)) {
      errors.push({
        pointer: `/movement/${hi}`,
        message: `movement speeds must satisfy prone < crouch_walk < crouch < run < sprint and prone < walk < run (${lo}=${a} >= ${hi}=${b})`,
      });
    }
  }
}

function checkVehicle(doc, errors) {
  const e = doc.gameplay?.gun_elevation_deg;
  if (Array.isArray(e) && e.length === 2 && num(e[0]) && num(e[1]) && e[0] >= e[1]) {
    errors.push({ pointer: '/gameplay/gun_elevation_deg', message: 'gun_elevation_deg must be [min,max] with min < max' });
  }
}

function checkMapMeta(doc, errors) {
  const h = doc.height;
  if (h && num(h.hmin_m) && num(h.hmax_m) && h.hmin_m >= h.hmax_m) {
    errors.push({ pointer: '/height', message: 'height.hmin_m must be < hmax_m' });
  }
}

function checkBots(doc, errors) {
  for (const [name, d] of Object.entries(doc.difficulties ?? {})) {
    const r = d?.reaction_ms;
    if (Array.isArray(r) && r.length === 2 && num(r[0]) && num(r[1]) && r[0] > r[1]) {
      errors.push({ pointer: `/difficulties/${name}/reaction_ms`, message: 'reaction_ms must be [min,max] with min <= max' });
    }
  }
}

export function checkRanges(doc, kind) {
  const errors = [];
  if (kind === 'weapon') checkWeapon(doc, errors);
  else if (kind === 'role') checkRole(doc, errors);
  else if (kind === 'vehicle') checkVehicle(doc, errors);
  else if (kind === 'map_meta') checkMapMeta(doc, errors);
  else if (kind === 'bots') checkBots(doc, errors);
  return errors;
}
