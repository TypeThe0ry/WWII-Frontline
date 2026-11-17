// Unit-suffix rule (ARCHITECTURE 8.5 / GAMEDESIGN 0.2): every numeric field carries a
// whitelisted unit suffix; known-wrong units (kph, ft, ...) get a specific message.

export const UNIT_SUFFIXES = [
  '_m', '_mm', '_ms', '_s', '_mps', '_mps2', '_rpm', '_rpm_practical', '_kg', '_deg', '_deg_s', '_hp',
  '_per_m', '_per_s', '_nm', '_n_per_m', '_ns_per_m', '_mult',
];

export const BAD_UNIT_SUFFIXES = [
  '_kph', '_mph', '_kmh', '_km', '_cm', '_ft', '_in', '_yd', '_lb', '_lbs', '_g', '_sec', '_min',
  '_rad', '_fps',
];

// Dimensionless numeric fields (counts, indices, coordinates, scores). Exact key names only.
export const DIMENSIONLESS = new Set([
  'count', 'ammo_mags', 'mag_reserve', 'magazine_capacity', 'pen_power', 'stamina_max',
  'max_per_squad', 'max_per_team', 'manpower_cost', 'squad_size_min', 'crew', 'rays_per_side',
  'manpower', 'munitions', 'fuel', 'friction_forward', 'friction_lateral',
  'epsg', 'samples', 'origin_e', 'origin_n', 'e', 'n', 'dx', 'dy', 'design', 'pos', 'mult',
]);

/** Container keys whose numeric children share the unit (part_mult.head, armor_mm.front). */
const hasUnit = (k) => UNIT_SUFFIXES.some((s) => k.endsWith(s));

export function checkUnits(doc) {
  const errors = [];
  const walk = (node, key, pointer, inUnit) => {
    if (typeof node === 'number') {
      if (inUnit || DIMENSIONLESS.has(key)) return;
      if (UNIT_SUFFIXES.some((s) => key.endsWith(s))) return;
      const bad = BAD_UNIT_SUFFIXES.find((s) => key.endsWith(s));
      errors.push({
        pointer,
        message: bad
          ? `unit suffix '${bad}' not allowed on '${key}' (use SI units: ${UNIT_SUFFIXES.join(' ')})`
          : `missing unit suffix on numeric field '${key}' (allowed: ${UNIT_SUFFIXES.join(' ')})`,
      });
    } else if (Array.isArray(node)) {
      node.forEach((v, i) => walk(v, key, `${pointer}/${i}`, inUnit));
    } else if (node && typeof node === 'object') {
      for (const [k, v] of Object.entries(node)) {
        if (k.startsWith('_')) continue; // _meta / _source / _expect_error
        walk(v, k, `${pointer}/${k}`, inUnit || (v !== null && typeof v === 'object' && hasUnit(k)));
      }
    }
  };
  walk(doc, '', '', false);
  return errors;
}
