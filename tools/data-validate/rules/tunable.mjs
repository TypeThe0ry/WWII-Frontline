// _meta rules: status present, tunable paths resolve, historical block cites a source.

export const STATUSES = ['initial', 'reviewed', 'measured'];

/** Kinds whose files must carry `_meta` (map files may omit it, see data/README.md). */
export const META_REQUIRED = new Set(['weapon', 'explosive', 'melee', 'ammo', 'gadget', 'rule_capture', 'rule_spawn', 'rule_supplies', 'role', 'loadout', 'vehicle', 'rules', 'bots']);

export function resolvePath(doc, dotted) {
  let cur = doc;
  for (const seg of dotted.split('.')) {
    if (cur === null || typeof cur !== 'object') return { found: false };
    const key = Array.isArray(cur) ? Number(seg) : seg;
    if (!Object.prototype.hasOwnProperty.call(cur, key)) return { found: false };
    cur = cur[key];
  }
  return { found: true, value: cur };
}

export function checkTunable(doc, kind) {
  const errors = [];
  const meta = doc._meta;
  if (meta === undefined) {
    if (META_REQUIRED.has(kind)) errors.push({ pointer: '', message: 'missing: _meta' });
  } else if (meta === null || typeof meta !== 'object') {
    errors.push({ pointer: '/_meta', message: '_meta must be an object' });
  } else {
    if (meta.status === undefined) {
      errors.push({ pointer: '/_meta', message: 'missing: _meta.status' });
    } else if (!STATUSES.includes(meta.status)) {
      errors.push({
        pointer: '/_meta/status',
        message: `invalid _meta.status '${meta.status}' (allowed: ${STATUSES.join(', ')})`,
      });
    }
    if (Array.isArray(meta.tunable)) {
      meta.tunable.forEach((p, i) => {
        if (typeof p !== 'string' || !resolvePath(doc, p).found) {
          errors.push({
            pointer: `/_meta/tunable/${i}`,
            message: `tunable path not found: ${p}`,
          });
        }
      });
    }
  }
  // historical values need a source (or the explicit "未核实" marker on the value).
  // Enforcement of status=reviewed is M0-031's job.
  if (doc.historical && typeof doc.historical === 'object') {
    const src = doc.historical._source;
    const ok = Array.isArray(src) ? src.length > 0 : typeof src === 'string' && src.trim() !== '';
    if (!ok) {
      errors.push({ pointer: '/historical', message: 'missing: historical._source' });
    }
  }
  return errors;
}
