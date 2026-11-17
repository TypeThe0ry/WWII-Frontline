// Drift statistics for M0-029 (pure functions; unit-tested in stats.test.mjs).

/** Root mean square of a numeric array (0 for empty). */
export function rms(xs) {
  if (xs.length === 0) return 0;
  let s = 0;
  for (const x of xs) s += x * x;
  return Math.sqrt(s / xs.length);
}

/** Least-squares slope of y over t (units of y per unit of t); 0 when t has no spread. */
export function slope(ts, ys) {
  const n = ts.length;
  if (n < 2) return 0;
  let mt = 0;
  let my = 0;
  for (let i = 0; i < n; i++) {
    mt += ts[i];
    my += ys[i];
  }
  mt /= n;
  my /= n;
  let num = 0;
  let den = 0;
  for (let i = 0; i < n; i++) {
    num += (ts[i] - mt) * (ys[i] - my);
    den += (ts[i] - mt) ** 2;
  }
  return den === 0 ? 0 : num / den;
}

/** 3D euclidean distance per tick between two interleaved xyz Float32Arrays (computed in f64). */
export function posErrors(a, b) {
  if (a.length !== b.length || a.length % 3 !== 0) throw new Error(`position arrays differ in length (${a.length} vs ${b.length})`);
  const n = a.length / 3;
  const e = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    const dx = a[3 * i] - b[3 * i];
    const dy = a[3 * i + 1] - b[3 * i + 1];
    const dz = a[3 * i + 2] - b[3 * i + 2];
    e[i] = Math.sqrt(dx * dx + dy * dy + dz * dz);
  }
  return e;
}

/**
 * Drift metrics of an error series sampled once per tick.
 * `cm_per_s` is the least-squares slope of err(t) x 100 (the M0-029 definition of "drift").
 */
export function driftMetrics(err, hz = 30, threshold = 0.05, segmentSeconds = 60) {
  const n = err.length;
  const t = new Float64Array(n);
  let max = 0;
  let over = 0;
  for (let i = 0; i < n; i++) {
    t[i] = (i + 1) / hz;
    if (err[i] > max) max = err[i];
    if (err[i] > threshold) over++;
  }
  const seg = Math.round(segmentSeconds * hz);
  const segment_max_m = [];
  for (let s = 0; s < n; s += seg) {
    let m = 0;
    for (let i = s; i < Math.min(n, s + seg); i++) m = Math.max(m, err[i]);
    segment_max_m.push(m);
  }
  return {
    rms_m: rms(err),
    max_m: max,
    cm_per_s: slope(t, err) * 100,
    over_0_05m_ticks: over,
    over_0_05m_ratio: n === 0 ? 0 : over / n,
    segment_max_m,
  };
}
