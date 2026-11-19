/**
 * Terrain shaders, GLSL ES 3.00 (WebGL2 path). Babylon converts the ES 1.0 style `attribute`/`varying`/
 * `gl_FragColor` spellings; `texelFetch` needs the ES 3.00 header it adds for WebGL2.
 * The vertex logic must mirror `morphVertexHeight` in cdlod-quadtree.ts and `TerrainData.heightAt` in
 * terrain-data.ts; terrain-material.wgsl.ts is the WebGPU twin of this file.
 *
 * Heights are stored as the raw little-endian u16 of the .r16 files uploaded as an RG8 texture
 * (R = low byte, G = high byte), decoded exactly and bilinearly filtered by hand (no float-texture filtering needed).
 */
import { SHADOW_RECEIVER_GLSL } from '../lighting/csm-shader';
import { DETAIL_FADE_M, MACRO_FADE_M, TRIPLANAR_FADE_M, TRIPLANAR_RANGE_DEG, TRIPLANAR_START_DEG } from './splat-math';

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
  vNormal = normalize(vec3((hl - hr) * uYScale, 2.0 * e, (hd - hu) * uYScale));

  vWorld = vec3(p.x, h * uYScale, p.y);
  vInfo = vec4(lod, k, 0.0, 0.0);
  gl_Position = viewProjection * vec4(vWorld, 1.0);
}
`;

export const TERRAIN_FRAGMENT_GLSL = /* glsl */ `
precision highp float;
precision highp int;

varying vec3 vWorld;
varying vec3 vNormal;
varying vec4 vInfo;

uniform vec2 uHeightRange;
uniform float uDebug; // 0 shaded, 1 lod, 2 morph, 3 normal, 4 height, 5 flat
uniform vec4 uSplat; // (texture grid extent m, splat enabled, max layers, macro mip)
uniform sampler2D splatA;
uniform sampler2D splatB;
uniform vec3 uCam;
${SHADOW_RECEIVER_GLSL}

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
  // Unsigned wraparound and an exact 24-bit conversion keep the hash stable across backends.
  uvec2 q = uvec2(ivec2(p));
  uint h = (q.x * 0x9e3779b1u) ^ (q.y * 0x85ebca6bu);
  h = (h ^ (h >> 15u)) * 0x2c1b3c6du;
  return float((h ^ (h >> 12u)) >> 8u) * (1.0 / 16777216.0);
}

float vnoise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash21(i), hash21(i + vec2(1.0, 0.0)), u.x), mix(hash21(i + vec2(0.0, 1.0)), hash21(i + vec2(1.0, 1.0)), u.x), u.y);
}

// Two-scale detail so layers do not read as flat colour (placeholder for the real KTX2 layers, PI-113).
float detail2(vec2 p, float period) {
  return 0.55 * vnoise(p / period) + 0.45 * vnoise(p / (period * 4.4));
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

vec3 shadeSplat(vec3 world, vec3 n, vec4 a, vec4 b, float dist) {
  float weights[8];
  weights[0] = a.x; weights[1] = a.y; weights[2] = a.z; weights[3] = a.w;
  weights[4] = b.x; weights[5] = b.y; weights[6] = b.z; weights[7] = b.w;
  // Low keeps the four largest weights (equal weights prefer lower indices).
  bool low = uSplat.z < 5.0;
  float detail = 1.0 - smoothstep(${DETAIL_FADE_M[0]}.0, ${DETAIL_FADE_M[1]}.0, dist);
  float steep = 0.0;
  if (dist < ${TRIPLANAR_FADE_M[1]}.0) {
    float slopeDeg = degrees(acos(clamp(n.y, -1.0, 1.0)));
    steep = smoothstep(${TRIPLANAR_START_DEG}.0, ${TRIPLANAR_START_DEG + TRIPLANAR_RANGE_DEG}.0, slopeDeg)
      * (1.0 - smoothstep(${TRIPLANAR_FADE_M[0]}.0, ${TRIPLANAR_FADE_M[1]}.0, dist));
  }
  vec3 axes = n * n;
  axes *= axes;
  axes /= axes.x + axes.y + axes.z;

  vec3 color = vec3(0.0);
  float sum = 0.0;
  for (int pick = 0; pick < 8; pick++) {
    int layer = pick;
    if (low) {
      if (pick >= 4) break;
      layer = 0;
      for (int k = 1; k < 8; k++) {
        if (weights[k] > weights[layer]) layer = k;
      }
    }
    float weight = weights[layer];
    if (weight <= 0.0) continue;
    weights[layer] = 0.0;
    vec3 layerBase = layerColor(layer);
    if (detail > 0.0) {
      float period = 0.70 + 0.13 * float(layer);
      float planar = detail2(world.xz, period);
      float d = planar;
      if (steep > 0.0) {
        float tri = axes.y * planar + axes.x * detail2(world.zy + 17.0, period) + axes.z * detail2(world.xy + 41.0, period);
        d = mix(planar, tri, steep);
      }
      layerBase *= 1.0 + detail * (d - 0.5) * (layer == 4 ? 0.45 : 0.30);
    }
    color += weight * layerBase;
    sum += weight;
  }
  // Bilinear interpolation of RGBA8 weights can drift from a unit sum. All-zero data falls back to grass.
  if (sum > 0.0) color /= sum;
  else color = layerColor(0);
  return color * (0.92 + 0.16 * vnoise(world.xz / 80.0));
}

float gridLine(vec2 p, float spacing, float width) {
  vec2 q = p / spacing;
  vec2 fw = fwidth(q);
  vec2 g = abs(fract(q - 0.5) - 0.5) / max(fw, vec2(1e-5));
  return 1.0 - clamp(min(g.x, g.y) / width, 0.0, 1.0);
}

void main() {
  vec3 n = normalize(vNormal);
  float l16 = gridLine(vWorld.xz, 16.0, 1.0);
  float l64 = gridLine(vWorld.xz, 64.0, 1.5);
  int lod = int(vInfo.x + 0.5);

  float slope = 1.0 - n.y;
  float hn = clamp((vWorld.y - uHeightRange.x) / max(uHeightRange.y - uHeightRange.x, 1e-3), 0.0, 1.0);
  vec3 base = vec3(0.0);
  if (uSplat.y > 0.5) {
    float dist = length(vWorld - uCam);
    float mip = smoothstep(${MACRO_FADE_M[0]}.0, ${MACRO_FADE_M[1]}.0, dist) * uSplat.w;
    vec2 uv = vWorld.xz / uSplat.x; // sampler clamps to the grid edge
    base = shadeSplat(vWorld, n, textureLod(splatA, uv, mip), textureLod(splatB, uv, mip), dist);
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
  float diff = max(dot(n, sun), 0.0) * shadowFactor(vWorld, n, uCam);
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
