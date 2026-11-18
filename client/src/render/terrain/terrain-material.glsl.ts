/**
 * Terrain shaders, GLSL ES 3.00 (WebGL2 path). Babylon converts the ES 1.0 style `attribute`/`varying`/
 * `gl_FragColor` spellings; `texelFetch` needs the ES 3.00 header it adds for WebGL2.
 * The vertex logic must mirror `morphVertexHeight` in cdlod-quadtree.ts and `TerrainData.heightAt` in
 * terrain-data.ts; terrain-material.wgsl.ts is the WebGPU twin of this file.
 *
 * Heights are stored as the raw little-endian u16 of the .r16 files uploaded as an RG8 texture
 * (R = low byte, G = high byte), decoded exactly and bilinearly filtered by hand (no float-texture filtering needed).
 */
export const TERRAIN_VERTEX_GLSL = /* glsl */ `
precision highp float;
precision highp int;
precision highp sampler2D;

attribute vec3 position;   // (i, 0, j) grid coordinates 0..64
attribute vec4 inst0;      // origin x, origin z, node size, lod
attribute vec4 inst1;      // coarser-neighbour mask, finer-neighbour mask, 0, 0

uniform mat4 viewProjection;
uniform vec3 uCam;
uniform vec2 uHeightRange;  // hmin, hmax
uniform vec4 uMap;          // near size (m), near samples, far origin (m), far res (m)
uniform vec2 uMap2;         // far samples, near/far blend width (m)
uniform float uYScale;      // debug vertical exaggeration, 1 = off
uniform sampler2D heightNear;
uniform sampler2D heightFar;

varying vec3 vWorld;
varying vec3 vNormal;
varying vec4 vInfo;         // lod, morph k, 0, 0

float rawU16(sampler2D t, ivec2 p) {
  vec2 v = texelFetch(t, p, 0).rg;
  return floor(v.r * 255.0 + 0.5) + 256.0 * floor(v.g * 255.0 + 0.5);
}

float bilinearU16(sampler2D t, vec2 uv, float n) {
  uv = clamp(uv, vec2(0.0), vec2(n - 1.0));
  vec2 f0 = min(floor(uv), vec2(n - 2.0));
  vec2 f = uv - f0;
  ivec2 i0 = ivec2(f0);
  float a = rawU16(t, i0);
  float b = rawU16(t, i0 + ivec2(1, 0));
  float c = rawU16(t, i0 + ivec2(0, 1));
  float d = rawU16(t, i0 + ivec2(1, 1));
  return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

float dequant(float u) {
  return uHeightRange.x + u * ((uHeightRange.y - uHeightRange.x) / 65535.0);
}

// Surface height at world (x,z): near map (1 m) inside the block, far map (8 m) outside, blended over uMap2.y metres.
float heightAt(vec2 p) {
  float nearH = dequant(bilinearU16(heightNear, p, uMap.y));
  float d = max(max(max(-p.x, p.x - uMap.x), max(-p.y, p.y - uMap.x)), 0.0);
  if (d <= 0.0) return nearH;
  float farH = dequant(bilinearU16(heightFar, (p - vec2(uMap.z)) / uMap.w, uMap2.x));
  float t = clamp(d / uMap2.y, 0.0, 1.0);
  float w = 1.0 - t * t * (3.0 - 2.0 * t);
  return mix(farH, nearH, w);
}

bool bit(int mask, int b) { return (mask & b) != 0; }

void main() {
  vec2 origin = inst0.xy;
  float size = inst0.z;
  float lod = inst0.w;
  int coarser = int(inst1.x + 0.5);
  int finer = int(inst1.y + 0.5);
  float cell = size / 64.0;
  vec2 ij = position.xz;
  vec2 p = origin + ij * cell;
  float h = heightAt(p);

  bool oddI = mod(ij.x, 2.0) > 0.5;
  bool oddJ = mod(ij.y, 2.0) > 0.5;
  float k = 0.0;
  if ((oddI || oddJ) && lod < 5.5) {
    bool onW = ij.x < 0.5;
    bool onE = ij.x > 63.5;
    bool onS = ij.y < 0.5;
    bool onN = ij.y > 63.5;
    if ((onW && bit(finer, 1)) || (onE && bit(finer, 2)) || (onS && bit(finer, 4)) || (onN && bit(finer, 8))) {
      k = 0.0;
    } else if ((onW && bit(coarser, 1)) || (onE && bit(coarser, 2)) || (onS && bit(coarser, 4)) || (onN && bit(coarser, 8))) {
      k = 1.0;
    } else {
      float r = 110.0 * float(1 << int(lod + 0.5));
      float dist = length(vec3(p.x, h, p.y) - uCam);
      k = smoothstep(0.7 * r, r, dist);
    }
    if (k > 0.0) {
      vec2 da;
      vec2 db;
      if (oddI && !oddJ) { da = vec2(-cell, 0.0); db = vec2(cell, 0.0); }
      else if (!oddI && oddJ) { da = vec2(0.0, -cell); db = vec2(0.0, cell); }
      else { da = vec2(-cell, -cell); db = vec2(cell, cell); }
      float hm = 0.5 * (heightAt(p + da) + heightAt(p + db));
      h = mix(h, hm, k);
    }
  }

  // Normal by central differences, widened with the morph so it stays smooth across LOD changes.
  float e = cell * (1.0 + k);
  float hl = heightAt(p - vec2(e, 0.0));
  float hr = heightAt(p + vec2(e, 0.0));
  float hd = heightAt(p - vec2(0.0, e));
  float hu = heightAt(p + vec2(0.0, e));
  vNormal = normalize(vec3(hl - hr, 2.0 * e, hd - hu));

  vWorld = vec3(p.x, h * uYScale, p.y);
  vInfo = vec4(lod, k, 0.0, 0.0);
  gl_Position = viewProjection * vec4(vWorld, 1.0);
}
`;

export const TERRAIN_FRAGMENT_GLSL = /* glsl */ `
precision highp float;

varying vec3 vWorld;
varying vec3 vNormal;
varying vec4 vInfo;

uniform vec2 uHeightRange;
uniform float uDebug; // 0 shaded, 1 lod, 2 morph, 3 normal, 4 height, 5 flat
uniform vec4 uSplat; // (map size m, splat enabled, 0, 0)
uniform sampler2D splatA;
uniform sampler2D splatB;

vec3 lodColor(int l) {
  if (l == 0) return vec3(0.90, 0.25, 0.25);
  if (l == 1) return vec3(0.95, 0.60, 0.20);
  if (l == 2) return vec3(0.95, 0.90, 0.25);
  if (l == 3) return vec3(0.35, 0.80, 0.30);
  if (l == 4) return vec3(0.25, 0.75, 0.85);
  if (l == 5) return vec3(0.30, 0.40, 0.90);
  return vec3(0.75, 0.35, 0.85);
}

float hash21(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float vnoise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash21(i), hash21(i + vec2(1.0, 0.0)), u.x), mix(hash21(i + vec2(0.0, 1.0)), hash21(i + vec2(1.0, 1.0)), u.x), u.y);
}

// Two-scale detail so layers do not read as flat colour (placeholder for the real KTX2 layers, PI-113).
float detail2(vec2 p) {
  return 0.55 * vnoise(p / 0.7) + 0.45 * vnoise(p / 3.1);
}

vec3 layerColor(int i) {
  if (i == 0) return vec3(0.27, 0.40, 0.17);
  if (i == 1) return vec3(0.46, 0.43, 0.22);
  if (i == 2) return vec3(0.45, 0.36, 0.24);
  if (i == 3) return vec3(0.27, 0.21, 0.15);
  if (i == 4) return vec3(0.47, 0.45, 0.42);
  if (i == 5) return vec3(0.55, 0.52, 0.46);
  if (i == 6) return vec3(0.70, 0.62, 0.44);
  return vec3(0.17, 0.20, 0.10);
}

float gridLine(vec2 p, float spacing, float width) {
  vec2 q = p / spacing;
  vec2 fw = fwidth(q);
  vec2 g = abs(fract(q - 0.5) - 0.5) / max(fw, vec2(1e-5));
  return 1.0 - clamp(min(g.x, g.y) / width, 0.0, 1.0);
}

void main() {
  vec3 n = normalize(vNormal);
  vec2 sUv = clamp(vWorld.xz / uSplat.x, vec2(0.0), vec2(1.0));
  vec4 sa = texture2D(splatA, sUv);
  vec4 sb = texture2D(splatB, sUv);
  float l16 = gridLine(vWorld.xz, 16.0, 1.0);
  float l64 = gridLine(vWorld.xz, 64.0, 1.5);
  int lod = int(vInfo.x + 0.5);

  float slope = 1.0 - n.y;
  float hn = clamp((vWorld.y - uHeightRange.x) / max(uHeightRange.y - uHeightRange.x, 1e-3), 0.0, 1.0);
  vec3 base = vec3(0.0);
  if (uSplat.y > 0.5) {
    float wts[8];
    wts[0] = sa.x; wts[1] = sa.y; wts[2] = sa.z; wts[3] = sa.w; wts[4] = sb.x; wts[5] = sb.y; wts[6] = sb.z; wts[7] = sb.w;
    vec3 an = pow(abs(n), vec3(4.0));
    vec3 tri = an / max(an.x + an.y + an.z, 1e-4);
    float steep = 1.0 - smoothstep(0.643, 0.766, n.y);
    float dPlanar = detail2(vWorld.xz);
    float dTri = tri.y * detail2(vWorld.xz) + tri.x * detail2(vWorld.zy + 17.0) + tri.z * detail2(vWorld.xy + 41.0);
    float d = mix(dPlanar, dTri, steep);
    for (int i = 0; i < 8; i++) {
      float amp = 0.30;
      if (i == 4) amp = 0.45;
      if (i == 5) amp = 0.38;
      base += wts[i] * layerColor(i) * (1.0 - 0.5 * amp + amp * d);
    }
    base *= 0.92 + 0.16 * vnoise(vWorld.xz / 80.0);
  } else {
    vec3 grass = vec3(0.27, 0.40, 0.17);
    vec3 dirt = vec3(0.45, 0.36, 0.24);
    vec3 rock = vec3(0.47, 0.45, 0.42);
    base = mix(grass, dirt, smoothstep(0.02, 0.10, slope) * 0.8 + 0.25 * hn);
    base = mix(base, rock, smoothstep(0.12, 0.30, slope));
    float blotch = 0.5 + 0.5 * sin(vWorld.x * 0.37) * sin(vWorld.z * 0.41);
    base *= 0.9 + 0.1 * blotch;
  }

  vec3 sun = normalize(vec3(-0.45, 0.80, -0.35));
  float diff = max(dot(n, sun), 0.0);
  vec3 lit = base * (0.32 + 0.78 * diff);
  lit = mix(lit, vec3(0.85), l16 * 0.10);
  lit = mix(lit, vec3(1.0, 0.92, 0.55), l64 * 0.30);

  vec3 col = lit;
  if (uDebug > 4.5) col = vec3(1.0);
  else if (uDebug > 3.5) col = vec3(hn);
  else if (uDebug > 2.5) col = n * 0.5 + 0.5;
  else if (uDebug > 1.5) col = vec3(vInfo.y, 1.0 - vInfo.y, 0.15);
  else if (uDebug > 0.5) col = lodColor(lod) * (0.45 + 0.55 * diff) * (1.0 - 0.25 * l16);
  gl_FragColor = vec4(col, 1.0);
}
`;
