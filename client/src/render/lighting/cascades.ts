/**
 * Cascade fitting for the custom CSM (PI-117 follow-up): pure maths, no engine. For each cascade the camera frustum slice
 * [near_i, far_i] is bounded by a sphere (rotation-stable), the light looks along `lightDir` from behind that sphere, and the
 * orthographic window is the sphere's diameter. The window centre is snapped to whole shadow texels in light space, so the shadow
 * edges do not swim when the camera moves.
 */
export type V3 = readonly [number, number, number];

export interface CameraPose {
  pos: V3;
  /** Unit basis. */
  forward: V3;
  right: V3;
  up: V3;
  /** Vertical FOV (rad) and aspect (width / height). */
  fov: number;
  aspect: number;
}

export interface Cascade {
  /** Sphere around the frustum slice. */
  center: V3;
  radius: number;
  /** Light-space basis (unit): the light travels along `dir`. */
  dir: V3;
  lightRight: V3;
  lightUp: V3;
  /** Texel-snapped centre actually used for the window. */
  snappedCenter: V3;
  /** World size of one shadow texel. */
  texel: number;
}

const dot = (a: V3, b: V3): number => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a: V3, b: V3): V3 => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const norm = (a: V3): V3 => {
  const l = Math.hypot(a[0], a[1], a[2]) || 1;
  return [a[0] / l, a[1] / l, a[2] / l];
};
const add = (a: V3, b: V3, s = 1): V3 => [a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s];
const dist = (a: V3, b: V3): number => Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);

/** The 8 corners of the frustum slice between view distances `near` and `far`. */
export function sliceCorners(c: CameraPose, near: number, far: number): V3[] {
  const th = Math.tan(c.fov / 2);
  const out: V3[] = [];
  for (const d of [near, far]) {
    const hh = d * th;
    const hw = hh * c.aspect;
    const centre = add(c.pos, c.forward, d);
    for (const [sx, sy] of [[-1, -1], [1, -1], [1, 1], [-1, 1]] as const) out.push(add(add(centre, c.right, sx * hw), c.up, sy * hh));
  }
  return out;
}

/** Smallest practical bounding sphere of the slice: centre on the view axis, radius to the farthest corner (rotation invariant). */
export function sliceSphere(c: CameraPose, near: number, far: number): { center: V3; radius: number } {
  const corners = sliceCorners(c, near, far);
  // the centre that equalises the distance to the near and far corners lies on the axis; solve for its depth t
  const th = Math.tan(c.fov / 2);
  const kn = near * near * (1 + th * th * (1 + c.aspect * c.aspect));
  const kf = far * far * (1 + th * th * (1 + c.aspect * c.aspect));
  let t = (kf - kn) / (2 * (far - near));
  t = Math.min(Math.max(t, near), far);
  const center = add(c.pos, c.forward, t);
  let radius = 0;
  for (const p of corners) radius = Math.max(radius, dist(center, p));
  return { center, radius: Math.ceil(radius * 16) / 16 }; // rounded up so the radius does not flicker
}

export function fitCascade(cam: CameraPose, near: number, far: number, lightDir: V3, mapSize: number): Cascade {
  const dir = norm(lightDir);
  const ref: V3 = Math.abs(dir[1]) > 0.99 ? [1, 0, 0] : [0, 1, 0];
  const lightRight = norm(cross(ref, dir));
  const lightUp = cross(dir, lightRight);
  const { center, radius } = sliceSphere(cam, near, far);
  const texel = (2 * radius) / mapSize;
  // snap the centre to the texel grid in the light's right/up plane (the depth axis needs no snapping)
  const x = Math.round(dot(center, lightRight) / texel) * texel;
  const y = Math.round(dot(center, lightUp) / texel) * texel;
  const z = dot(center, dir);
  const snapped = add(add(add([0, 0, 0], lightRight, x), lightUp, y), dir, z);
  return { center, radius, dir, lightRight, lightUp, snappedCenter: snapped, texel };
}

/** Light view-projection (column-major 4x4, row-vector-free: clip = M * [p, 1]) for a cascade; depth range [0,1] when `halfZ`, else [-1,1]. */
export function cascadeMatrix(c: Cascade, depthBack: number, halfZ: boolean): Float32Array {
  const r = c.radius;
  const far = depthBack + 2 * r;
  // eye = centre - dir * (r + depthBack): everything inside the sphere, plus casters up to depthBack behind it, is in front of the eye
  const eye = add(c.snappedCenter, c.dir, -(r + depthBack));
  const f = c.dir;
  const s = c.lightRight;
  const u = c.lightUp;
  const m = new Float32Array(16);
  // view: rows s, u, f (left-handed: +z forward); orthographic: x,y in [-r, r] -> [-1, 1]
  const sx = 1 / r;
  const dz = halfZ ? 1 / far : 2 / far;
  const oz = halfZ ? 0 : -1;
  const row = (a: V3, k: number, off: number): [number, number, number, number] => [a[0] * k, a[1] * k, a[2] * k, -dot(a, eye) * k + off];
  const r0 = row(s, sx, 0);
  const r1 = row(u, sx, 0);
  const r2 = row(f, dz, oz);
  const rows = [r0, r1, r2, [0, 0, 0, 1] as [number, number, number, number]];
  for (let rIdx = 0; rIdx < 4; rIdx++) for (let col = 0; col < 4; col++) m[col * 4 + rIdx] = rows[rIdx]![col]!;
  return m;
}

/** Applies a column-major matrix to a point (for tests / CPU lookups). */
export function transformPoint(m: Float32Array, p: V3): [number, number, number, number] {
  const o: [number, number, number, number] = [0, 0, 0, 0];
  for (let r = 0; r < 4; r++) o[r] = m[r]! * p[0] + m[4 + r]! * p[1] + m[8 + r]! * p[2] + m[12 + r]!;
  return o;
}

/** Index of the cascade covering view distance `d` (splits are the far edges), or -1 beyond the last. */
export function cascadeIndex(splits: readonly number[], d: number): number {
  for (let i = 0; i < splits.length; i++) if (d <= (splits[i] as number)) return i;
  return -1;
}
