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

const TANK_TYPES = ['light_tank', 'medium_tank', 'heavy_tank', 'tank_destroyer'];
const ARMOR_ZONES = ['hull_front_upper', 'hull_front_lower', 'hull_side', 'hull_rear', 'hull_top', 'turret_front', 'turret_side', 'turret_rear', 'turret_top'];

function checkVehicle(doc, errors) {
  const g0 = doc.gameplay ?? {};
  if (Array.isArray(g0.seats)) {
    const seen = new Set();
    g0.seats.forEach((s, i) => {
      if (seen.has(s?.id)) errors.push({ pointer: `/gameplay/seats/${i}/id`, message: `duplicate seat id: ${s.id}` });
      seen.add(s?.id);
    });
  }
  if (Array.isArray(g0.armor_zones) && TANK_TYPES.includes(doc.type)) {
    const ids = new Set(g0.armor_zones.map((z) => z?.id));
    for (const z of ARMOR_ZONES) {
      if (!ids.has(z)) errors.push({ pointer: '/gameplay/armor_zones', message: `missing armor zone: ${z}` });
    }
    if (ids.size !== g0.armor_zones.length) errors.push({ pointer: '/gameplay/armor_zones', message: 'duplicate armor zone id' });
  }
  for (const [list, name] of [[g0.modules, 'modules'], [g0.weapons, 'weapons']]) {
    if (!Array.isArray(list)) continue;
    const seen = new Set();
    list.forEach((m, i) => {
      if (seen.has(m?.id)) errors.push({ pointer: `/gameplay/${name}/${i}/id`, message: `duplicate ${name} id: ${m.id}` });
      seen.add(m?.id);
    });
  }
  if (Array.isArray(g0.weapons) && Array.isArray(g0.seats)) {
    const seatIds = new Set(g0.seats.map((s) => s?.id));
    g0.weapons.forEach((w, i) => {
      if (!seatIds.has(w?.seat)) errors.push({ pointer: `/gameplay/weapons/${i}/seat`, message: `weapon seat '${w?.seat}' is not a seat of this vehicle` });
    });
  }
  // GAMEDESIGN 14.3: game top speed = historical road speed x 0.75.
  const hv = doc.historical?.max_speed_mps;
  if (num(hv) && num(g0.max_speed_mps) && Math.abs(g0.max_speed_mps - hv * 0.75) > hv * 0.75 * 0.005) {
    errors.push({ pointer: '/gameplay/max_speed_mps', message: `max_speed_mps must be 0.75 x historical (${(hv * 0.75).toFixed(4)}), got ${g0.max_speed_mps}` });
  }
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

function checkEconomy(doc, errors) {
  for (const kind of ['manpower', 'munitions', 'fuel']) {
    const resource = doc[kind];
    if (num(resource?.initial) && num(resource.cap) && resource.initial > resource.cap) {
      errors.push({ pointer: `/${kind}/initial`, message: 'initial must be <= cap' });
    }
  }
}

function checkResourceNode(doc, errors) {
  if (typeof doc.kind === 'string' && typeof doc.id === 'string' && doc.id !== `resource_node_${doc.kind}`) {
    errors.push({ pointer: '/id', message: 'id must match resource_node_<kind>' });
  }
  if (num(doc.max_per_team_per_kind) && num(doc.max_per_team) && doc.max_per_team_per_kind > doc.max_per_team) {
    errors.push({ pointer: '/max_per_team_per_kind', message: 'max_per_team_per_kind must be <= max_per_team' });
  }
}

export function checkRanges(doc, kind) {
  const errors = [];
  if (kind === 'weapon') checkWeapon(doc, errors);
  else if (kind === 'role') checkRole(doc, errors);
  else if (kind === 'vehicle') checkVehicle(doc, errors);
  else if (kind === 'map_meta') checkMapMeta(doc, errors);
  else if (kind === 'bots') checkBots(doc, errors);
  else if (kind === 'economy') checkEconomy(doc, errors);
  else if (kind === 'resource_node') checkResourceNode(doc, errors);
  return errors;
}
