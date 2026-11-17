/** Tiny column-major linear algebra helpers (glTF matrix layout). */
export type Mat3 = readonly [number, number, number, number, number, number, number, number, number]; // column-major
export type Vec3 = readonly [number, number, number];

/** Signed axis names used for the source "up" / "forward" declaration. */
export type Axis = '+X' | '-X' | '+Y' | '-Y' | '+Z' | '-Z';

export function axisVector(a: Axis): Vec3 {
  const s = a[0] === '-' ? -1 : 1;
  return a[1] === 'X' ? [s, 0, 0] : a[1] === 'Y' ? [0, s, 0] : [0, 0, s];
}

export function cross(a: Vec3, b: Vec3): Vec3 {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}

export function dot(a: Vec3, b: Vec3): number {
  return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

/** 3x3 from three column vectors. */
export function fromColumns(c0: Vec3, c1: Vec3, c2: Vec3): Mat3 {
  return [c0[0], c0[1], c0[2], c1[0], c1[1], c1[2], c2[0], c2[1], c2[2]];
}

export function mul3(a: Mat3, b: Mat3): Mat3 {
  const o = new Array<number>(9).fill(0);
  for (let c = 0; c < 3; c++)
    for (let r = 0; r < 3; r++) {
      let s = 0;
      for (let k = 0; k < 3; k++) s += (a[k * 3 + r] as number) * (b[c * 3 + k] as number);
      o[c * 3 + r] = s;
    }
  return o as unknown as Mat3;
}

export function transpose3(a: Mat3): Mat3 {
  return [a[0], a[3], a[6], a[1], a[4], a[7], a[2], a[5], a[8]];
}

export function det3(m: Mat3): number {
  return (
    m[0] * (m[4] * m[8] - m[7] * m[5]) - m[3] * (m[1] * m[8] - m[7] * m[2]) + m[6] * (m[1] * m[5] - m[4] * m[2])
  );
}

/** Inverse-transpose of a 3x3 (normal matrix). Returns the identity for a singular matrix. */
export function inverseTranspose3(m: Mat3): Mat3 {
  const d = det3(m);
  if (Math.abs(d) < 1e-30) return [1, 0, 0, 0, 1, 0, 0, 0, 1];
  // inverse = adjugate / det, adjugate = cofactor^T, so inverse-transpose = cofactor / det.
  const g = (r: number, c: number): number => m[c * 3 + r] as number;
  const out = new Array<number>(9);
  for (let col = 0; col < 3; col++)
    for (let row = 0; row < 3; row++) {
      const [r0, r1] = [0, 1, 2].filter((x) => x !== row) as [number, number, number];
      const [c0, c1] = [0, 1, 2].filter((x) => x !== col) as [number, number, number];
      const minor = g(r0, c0) * g(r1, c1) - g(r0, c1) * g(r1, c0);
      out[col * 3 + row] = (((row + col) % 2 === 0 ? 1 : -1) * minor) / d;
    }
  return out as unknown as Mat3;
}

export function applyMat3(m: Mat3, x: number, y: number, z: number): Vec3 {
  return [m[0] * x + m[3] * y + m[6] * z, m[1] * x + m[4] * y + m[7] * z, m[2] * x + m[5] * y + m[8] * z];
}

/** Quaternion [x, y, z, w] of a proper rotation matrix. */
export function quatFromRotation(m: Mat3): [number, number, number, number] {
  const m00 = m[0],
    m10 = m[1],
    m20 = m[2],
    m01 = m[3],
    m11 = m[4],
    m21 = m[5],
    m02 = m[6],
    m12 = m[7],
    m22 = m[8];
  const tr = m00 + m11 + m22;
  let x: number, y: number, z: number, w: number;
  if (tr > 0) {
    const s = Math.sqrt(tr + 1) * 2;
    w = 0.25 * s;
    x = (m21 - m12) / s;
    y = (m02 - m20) / s;
    z = (m10 - m01) / s;
  } else if (m00 > m11 && m00 > m22) {
    const s = Math.sqrt(1 + m00 - m11 - m22) * 2;
    w = (m21 - m12) / s;
    x = 0.25 * s;
    y = (m01 + m10) / s;
    z = (m02 + m20) / s;
  } else if (m11 > m22) {
    const s = Math.sqrt(1 + m11 - m00 - m22) * 2;
    w = (m02 - m20) / s;
    x = (m01 + m10) / s;
    y = 0.25 * s;
    z = (m12 + m21) / s;
  } else {
    const s = Math.sqrt(1 + m22 - m00 - m11) * 2;
    w = (m10 - m01) / s;
    x = (m02 + m20) / s;
    y = (m12 + m21) / s;
    z = 0.25 * s;
  }
  const n = Math.hypot(x, y, z, w) || 1;
  return [x / n, y / n, z / n, w / n];
}

/**
 * Rotation taking a source frame (up axis, forward axis, right = forward x up, all right-handed) to the pipeline frame:
 * Y up, -Z forward, +X right (ARCHITECTURE §11.1 step 2).
 */
export function frameRotation(up: Axis, forward: Axis): Mat3 {
  const u = axisVector(up);
  const f = axisVector(forward);
  if (Math.abs(dot(u, f)) > 0.5) throw new Error(`up axis ${up} and forward axis ${forward} must be perpendicular`);
  const r = cross(f, u);
  const src = fromColumns(r, u, f);
  const dst = fromColumns([1, 0, 0], [0, 1, 0], [0, 0, -1]);
  return mul3(dst, transpose3(src));
}
