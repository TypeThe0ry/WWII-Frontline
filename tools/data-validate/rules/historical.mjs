// Rule `hist-source-required` (M0-031, REFERENCE_MEASUREMENTS 6): once `_meta.status` is
// `reviewed` or `measured`, every field under `historical` must either have a structured
// `_source` record (with a locator) or be listed in `_meta.unverified[]`
// (as `historical.<field>`), or carry the explicit "未核实" value. `initial` is not enforced.

export const REVIEWED_STATUSES = ['reviewed', 'measured'];

export function checkHistorical(doc, kind) {
  const errors = [];
  if (kind !== 'weapon' && kind !== 'vehicle') return errors;
  const meta = doc._meta;
  const hist = doc.historical;
  if (!meta || typeof meta !== 'object' || !REVIEWED_STATUSES.includes(meta.status)) return errors;
  if (!hist || typeof hist !== 'object') return errors;

  const rule = 'hist-source-required';
  const src = hist._source;
  const records = Array.isArray(src) ? src : [];
  if (!Array.isArray(src)) {
    errors.push({
      pointer: '/historical/_source',
      message: `${rule}: status '${meta.status}' needs a structured _source array`,
    });
  }
  const unverified = new Set(Array.isArray(meta.unverified) ? meta.unverified : []);
  const sourced = new Set();
  records.forEach((r, i) => {
    if (!r || typeof r !== 'object') return;
    if (typeof r.locator !== 'string' || r.locator.trim() === '') {
      errors.push({ pointer: `/historical/_source/${i}`, message: `${rule}: record '${r.field}' has no locator` });
    } else if (typeof r.field === 'string') {
      sourced.add(r.field);
    }
  });
  for (const key of Object.keys(hist)) {
    if (key.startsWith('_')) continue;
    if (hist[key] === '未核实') continue;
    if (sourced.has(key) || unverified.has(`historical.${key}`)) continue;
    errors.push({
      pointer: `/historical/${key}`,
      message: `${rule}: historical.${key} has no _source record and is not in _meta.unverified`,
    });
  }
  for (const [i, u] of [...unverified].entries()) {
    const key = typeof u === 'string' && u.startsWith('historical.') ? u.slice('historical.'.length) : null;
    if (key === null || !(key in hist)) {
      errors.push({ pointer: `/_meta/unverified/${i}`, message: `${rule}: unverified entry '${u}' does not name a historical field` });
    }
  }
  return errors;
}
