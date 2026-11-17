/**
 * Terrain shaders, WGSL (WebGPU path), Babylon ShaderMaterial conventions (`attribute`/`uniform`/`varying`
 * declarations, `vertexInputs`/`vertexOutputs`/`fragmentInputs`/`fragmentOutputs`).
 * Twin of terrain-material.glsl.ts: keep the two in step (and with `morphVertexHeight` / `TerrainData.heightAt`).
 * Heights come from RG8 textures holding the raw little-endian u16 (R = low byte, G = high byte), read with
 * `textureLoad` and filtered by hand.
 */
export const TERRAIN_VERTEX_WGSL = /* wgsl */ `
attribute position: vec3f;
attribute inst0: vec4f;
attribute inst1: vec4f;

uniform viewProjection: mat4x4f;
uniform uCam: vec3f;
uniform uHeightRange: vec2f;
uniform uMap: vec4f;
uniform uMap2: vec2f;
uniform uYScale: f32;

var heightNear: texture_2d<f32>;
var heightFar: texture_2d<f32>;

varying vWorld: vec3f;
varying vNormal: vec3f;
varying vInfo: vec4f;

fn rawU16(t: texture_2d<f32>, p: vec2i) -> f32 {
  let v = textureLoad(t, p, 0).rg;
  return floor(v.r * 255.0 + 0.5) + 256.0 * floor(v.g * 255.0 + 0.5);
}

fn bilinearU16(t: texture_2d<f32>, uvIn: vec2f, n: f32) -> f32 {
  let uv = clamp(uvIn, vec2f(0.0), vec2f(n - 1.0));
  let f0 = min(floor(uv), vec2f(n - 2.0));
  let f = uv - f0;
  let i0 = vec2i(f0);
  let a = rawU16(t, i0);
  let b = rawU16(t, i0 + vec2i(1, 0));
  let c = rawU16(t, i0 + vec2i(0, 1));
  let d = rawU16(t, i0 + vec2i(1, 1));
  return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

fn dequant(u: f32) -> f32 {
  return uniforms.uHeightRange.x + u * ((uniforms.uHeightRange.y - uniforms.uHeightRange.x) / 65535.0);
}

fn heightAt(p: vec2f) -> f32 {
  let nearH = dequant(bilinearU16(heightNear, p, uniforms.uMap.y));
  let d = max(max(max(-p.x, p.x - uniforms.uMap.x), max(-p.y, p.y - uniforms.uMap.x)), 0.0);
  if (d <= 0.0) {
    return nearH;
  }
  let farH = dequant(bilinearU16(heightFar, (p - vec2f(uniforms.uMap.z)) / uniforms.uMap.w, uniforms.uMap2.x));
  let t = clamp(d / uniforms.uMap2.y, 0.0, 1.0);
  let w = 1.0 - t * t * (3.0 - 2.0 * t);
  return mix(farH, nearH, w);
}

fn hasBit(mask: u32, b: u32) -> bool {
  return (mask & b) != 0u;
}

@vertex
fn main(input: VertexInputs) -> FragmentInputs {
  let origin = vertexInputs.inst0.xy;
  let size = vertexInputs.inst0.z;
  let lod = vertexInputs.inst0.w;
  let coarser = u32(vertexInputs.inst1.x + 0.5);
  let finer = u32(vertexInputs.inst1.y + 0.5);
  let cell = size / 64.0;
  let ij = vertexInputs.position.xz;
  let p = origin + ij * cell;
  var h = heightAt(p);

  let oddI = (ij.x % 2.0) > 0.5;
  let oddJ = (ij.y % 2.0) > 0.5;
  var k = 0.0;
  if ((oddI || oddJ) && lod < 5.5) {
    let onW = ij.x < 0.5;
    let onE = ij.x > 63.5;
    let onS = ij.y < 0.5;
    let onN = ij.y > 63.5;
    if ((onW && hasBit(finer, 1u)) || (onE && hasBit(finer, 2u)) || (onS && hasBit(finer, 4u)) || (onN && hasBit(finer, 8u))) {
      k = 0.0;
    } else if ((onW && hasBit(coarser, 1u)) || (onE && hasBit(coarser, 2u)) || (onS && hasBit(coarser, 4u)) || (onN && hasBit(coarser, 8u))) {
      k = 1.0;
    } else {
      let r = 110.0 * f32(1u << u32(lod + 0.5));
      let dist = length(vec3f(p.x, h, p.y) - uniforms.uCam);
      k = smoothstep(0.7 * r, r, dist);
    }
    if (k > 0.0) {
      var da = vec2f(-cell, -cell);
      var db = vec2f(cell, cell);
      if (oddI && !oddJ) {
        da = vec2f(-cell, 0.0);
        db = vec2f(cell, 0.0);
      } else if (!oddI && oddJ) {
        da = vec2f(0.0, -cell);
        db = vec2f(0.0, cell);
      }
      let hm = 0.5 * (heightAt(p + da) + heightAt(p + db));
      h = mix(h, hm, k);
    }
  }

  let e = cell * (1.0 + k);
  let hl = heightAt(p - vec2f(e, 0.0));
  let hr = heightAt(p + vec2f(e, 0.0));
  let hd = heightAt(p - vec2f(0.0, e));
  let hu = heightAt(p + vec2f(0.0, e));
  vertexOutputs.vNormal = normalize(vec3f(hl - hr, 2.0 * e, hd - hu));

  let world = vec3f(p.x, h * uniforms.uYScale, p.y);
  vertexOutputs.vWorld = world;
  vertexOutputs.vInfo = vec4f(lod, k, 0.0, 0.0);
  vertexOutputs.position = uniforms.viewProjection * vec4f(world, 1.0);
}
`;

export const TERRAIN_FRAGMENT_WGSL = /* wgsl */ `
uniform uHeightRange: vec2f;
uniform uDebug: f32;

varying vWorld: vec3f;
varying vNormal: vec3f;
varying vInfo: vec4f;

fn lodColor(l: i32) -> vec3f {
  if (l == 0) { return vec3f(0.90, 0.25, 0.25); }
  if (l == 1) { return vec3f(0.95, 0.60, 0.20); }
  if (l == 2) { return vec3f(0.95, 0.90, 0.25); }
  if (l == 3) { return vec3f(0.35, 0.80, 0.30); }
  if (l == 4) { return vec3f(0.25, 0.75, 0.85); }
  if (l == 5) { return vec3f(0.30, 0.40, 0.90); }
  return vec3f(0.75, 0.35, 0.85);
}

fn gridLine(p: vec2f, spacing: f32, width: f32) -> f32 {
  let q = p / spacing;
  let fw = fwidth(q);
  let g = abs(fract(q - 0.5) - 0.5) / max(fw, vec2f(1e-5));
  return 1.0 - clamp(min(g.x, g.y) / width, 0.0, 1.0);
}

@fragment
fn main(input: FragmentInputs) -> FragmentOutputs {
  // Derivative-based terms first (uniform control flow).
  let world = fragmentInputs.vWorld;
  let l16 = gridLine(world.xz, 16.0, 1.0);
  let l64 = gridLine(world.xz, 64.0, 1.5);
  let n = normalize(fragmentInputs.vNormal);
  let info = fragmentInputs.vInfo;
  let lod = i32(info.x + 0.5);

  let slope = 1.0 - n.y;
  let hn = clamp((world.y - uniforms.uHeightRange.x) / max(uniforms.uHeightRange.y - uniforms.uHeightRange.x, 1e-3), 0.0, 1.0);
  let grass = vec3f(0.27, 0.40, 0.17);
  let dirt = vec3f(0.45, 0.36, 0.24);
  let rock = vec3f(0.47, 0.45, 0.42);
  var base = mix(grass, dirt, smoothstep(0.02, 0.10, slope) * 0.8 + 0.25 * hn);
  base = mix(base, rock, smoothstep(0.12, 0.30, slope));
  let blotch = 0.5 + 0.5 * sin(world.x * 0.37) * sin(world.z * 0.41);
  base = base * (0.9 + 0.1 * blotch);

  let sun = normalize(vec3f(-0.45, 0.80, -0.35));
  let diff = max(dot(n, sun), 0.0);
  var lit = base * (0.32 + 0.78 * diff);
  lit = mix(lit, vec3f(0.85), l16 * 0.10);
  lit = mix(lit, vec3f(1.0, 0.92, 0.55), l64 * 0.30);

  var col = lit;
  if (uniforms.uDebug > 4.5) {
    col = vec3f(1.0);
  } else if (uniforms.uDebug > 3.5) {
    col = vec3f(hn);
  } else if (uniforms.uDebug > 2.5) {
    col = n * 0.5 + 0.5;
  } else if (uniforms.uDebug > 1.5) {
    col = vec3f(info.y, 1.0 - info.y, 0.15);
  } else if (uniforms.uDebug > 0.5) {
    col = lodColor(lod) * (0.45 + 0.55 * diff) * (1.0 - 0.25 * l16);
  }
  fragmentOutputs.color = vec4f(col, 1.0);
}
`;
