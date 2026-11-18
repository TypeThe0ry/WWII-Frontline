/**
 * Shader pieces of the custom CSM: (1) the caster depth pass (plain and thin-instanced), (2) the receiver snippets that the
 * terrain and PBR shaders paste in (GLSL and WGSL twins). Receivers sample up to 4 cascade depth maps (RGBA float render targets,
 * depth in R, NEAREST) with a manual 3x3 PCF and a normal-offset bias. Uniform contract (set by `custom-csm.ts`):
 *   uShadowVP0..3 (mat4)  world -> light clip of each cascade      uShadowSplits (vec4) far edge of each cascade (view distance)
 *   uShadowInfo   (vec4)  x = number of cascades (0 = off), y = flip V of the lookup (measured 0 on both backends, kept as a switch), z = depth is [0,1] already (1 on WebGPU)
 *   uShadowTexel  (vec4)  1 / map size of each cascade        uShadowWorld (vec4) world size of one texel of each cascade
 *   uShadowParams (vec4) x = depth bias, y = normal offset (in texels)
 *   samplers shadow0..shadow3
 */
export const SHADOW_UNIFORMS = ['uShadowVP0', 'uShadowVP1', 'uShadowVP2', 'uShadowVP3', 'uShadowSplits', 'uShadowInfo', 'uShadowTexel', 'uShadowWorld', 'uShadowParams'];
export const SHADOW_SAMPLERS = ['shadow0', 'shadow1', 'shadow2', 'shadow3'];

export const DEPTH_VERTEX_GLSL = (instanced: boolean): string => `
precision highp float;
attribute vec3 position;
${instanced ? 'attribute vec4 world0;\nattribute vec4 world1;\nattribute vec4 world2;\nattribute vec4 world3;' : 'uniform mat4 world;'}
uniform mat4 viewProjection;
void main() {
  ${instanced ? 'mat4 W = mat4(world0, world1, world2, world3);' : 'mat4 W = world;'}
  gl_Position = viewProjection * W * vec4(position, 1.0);
}
`;
export const DEPTH_FRAGMENT_GLSL = `
precision highp float;
void main() {
  gl_FragColor = vec4(gl_FragCoord.z, gl_FragCoord.z, gl_FragCoord.z, 1.0);
}
`;
export const DEPTH_VERTEX_WGSL = (instanced: boolean): string => `
attribute position: vec3f;
${instanced ? 'attribute world0: vec4f;\nattribute world1: vec4f;\nattribute world2: vec4f;\nattribute world3: vec4f;' : 'uniform world: mat4x4f;'}
uniform viewProjection: mat4x4f;
@vertex
fn main(input: VertexInputs) -> FragmentInputs {
  ${instanced ? 'let W = mat4x4f(vertexInputs.world0, vertexInputs.world1, vertexInputs.world2, vertexInputs.world3);' : 'let W = uniforms.world;'}
  vertexOutputs.position = uniforms.viewProjection * W * vec4f(vertexInputs.position, 1.0);
}
`;
export const DEPTH_FRAGMENT_WGSL = `
@fragment
fn main(input: FragmentInputs) -> FragmentOutputs {
  let z = fragmentInputs.position.z;
  fragmentOutputs.color = vec4f(z, z, z, 1.0);
}
`;

/** GLSL declarations + `shadowFactor(world, normal, camPos)` -> 0 (shadowed) .. 1 (lit). Paste before `main`. */
export const SHADOW_RECEIVER_GLSL = /* glsl */ `
uniform mat4 uShadowVP0;
uniform mat4 uShadowVP1;
uniform mat4 uShadowVP2;
uniform mat4 uShadowVP3;
uniform vec4 uShadowSplits;
uniform vec4 uShadowInfo;
uniform vec4 uShadowTexel;
uniform vec4 uShadowWorld;
uniform vec4 uShadowParams;
uniform sampler2D shadow0;
uniform sampler2D shadow1;
uniform sampler2D shadow2;
uniform sampler2D shadow3;

float shadowTap(sampler2D tex, vec2 uv, float depth) {
  return texture2D(tex, uv).r >= depth ? 1.0 : 0.0;
}

float shadowPcf(sampler2D tex, vec3 lp, float texel) {
  vec2 uv = lp.xy * 0.5 + 0.5;
  if (uShadowInfo.y > 0.5) uv.y = 1.0 - uv.y;
  float depth = (uShadowInfo.z > 0.5 ? lp.z : lp.z * 0.5 + 0.5) - uShadowParams.x;
  if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || depth > 1.0) return 1.0;
  float s = 0.0;
  for (int j = -1; j <= 1; j++) {
    for (int i = -1; i <= 1; i++) {
      s += shadowTap(tex, uv + vec2(float(i), float(j)) * texel, depth);
    }
  }
  return s / 9.0;
}

float shadowFactor(vec3 world, vec3 nrm, vec3 camPos) {
  if (uShadowInfo.x < 0.5) return 1.0;
  float d = length(world - camPos);
  float res = 1.0;
  if (d <= uShadowSplits.x) {
    vec3 p = world + nrm * (uShadowParams.y * uShadowWorld.x);
    vec4 lp = uShadowVP0 * vec4(p, 1.0);
    res = shadowPcf(shadow0, lp.xyz, uShadowTexel.x);
  } else if (d <= uShadowSplits.y && uShadowInfo.x > 1.5) {
    vec3 p = world + nrm * (uShadowParams.y * uShadowWorld.y);
    vec4 lp = uShadowVP1 * vec4(p, 1.0);
    res = shadowPcf(shadow1, lp.xyz, uShadowTexel.y);
  } else if (d <= uShadowSplits.z && uShadowInfo.x > 2.5) {
    vec3 p = world + nrm * (uShadowParams.y * uShadowWorld.z);
    vec4 lp = uShadowVP2 * vec4(p, 1.0);
    res = shadowPcf(shadow2, lp.xyz, uShadowTexel.z);
  } else if (d <= uShadowSplits.w && uShadowInfo.x > 3.5) {
    vec3 p = world + nrm * (uShadowParams.y * uShadowWorld.w);
    vec4 lp = uShadowVP3 * vec4(p, 1.0);
    res = shadowPcf(shadow3, lp.xyz, uShadowTexel.w);
  }
  return res;
}
`;

/** WGSL twin of the receiver (textures are read with `textureLoad`, so the cascade branch may be non-uniform). */
export const SHADOW_RECEIVER_WGSL = /* wgsl */ `
uniform uShadowVP0: mat4x4f;
uniform uShadowVP1: mat4x4f;
uniform uShadowVP2: mat4x4f;
uniform uShadowVP3: mat4x4f;
uniform uShadowSplits: vec4f;
uniform uShadowInfo: vec4f;
uniform uShadowTexel: vec4f;
uniform uShadowWorld: vec4f;
uniform uShadowParams: vec4f;
var shadow0: texture_2d<f32>;
var shadow1: texture_2d<f32>;
var shadow2: texture_2d<f32>;
var shadow3: texture_2d<f32>;

fn shadowTapLoad(tex: texture_2d<f32>, uv: vec2f, depth: f32) -> f32 {
  let dims = vec2f(textureDimensions(tex, 0));
  let ip = vec2i(clamp(uv * dims, vec2f(0.0), dims - vec2f(1.0)));
  return select(0.0, 1.0, textureLoad(tex, ip, 0).r >= depth);
}

fn shadowPcf(tex: texture_2d<f32>, lp: vec3f, texel: f32) -> f32 {
  var uv = lp.xy * 0.5 + vec2f(0.5);
  if (uniforms.uShadowInfo.y > 0.5) { uv.y = 1.0 - uv.y; }
  var z = lp.z * 0.5 + 0.5;
  if (uniforms.uShadowInfo.z > 0.5) { z = lp.z; }
  let depth = z - uniforms.uShadowParams.x;
  if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || depth > 1.0) { return 1.0; }
  var s = 0.0;
  for (var j = -1; j <= 1; j = j + 1) {
    for (var i = -1; i <= 1; i = i + 1) {
      s = s + shadowTapLoad(tex, uv + vec2f(f32(i), f32(j)) * texel, depth);
    }
  }
  return s / 9.0;
}

fn shadowFactor(world: vec3f, nrm: vec3f, camPos: vec3f) -> f32 {
  if (uniforms.uShadowInfo.x < 0.5) { return 1.0; }
  let d = length(world - camPos);
  var res = 1.0;
  if (d <= uniforms.uShadowSplits.x) {
    let p = world + nrm * (uniforms.uShadowParams.y * uniforms.uShadowWorld.x);
    let lp = uniforms.uShadowVP0 * vec4f(p, 1.0);
    res = shadowPcf(shadow0, lp.xyz, uniforms.uShadowTexel.x);
  } else if (d <= uniforms.uShadowSplits.y && uniforms.uShadowInfo.x > 1.5) {
    let p = world + nrm * (uniforms.uShadowParams.y * uniforms.uShadowWorld.y);
    let lp = uniforms.uShadowVP1 * vec4f(p, 1.0);
    res = shadowPcf(shadow1, lp.xyz, uniforms.uShadowTexel.y);
  } else if (d <= uniforms.uShadowSplits.z && uniforms.uShadowInfo.x > 2.5) {
    let p = world + nrm * (uniforms.uShadowParams.y * uniforms.uShadowWorld.z);
    let lp = uniforms.uShadowVP2 * vec4f(p, 1.0);
    res = shadowPcf(shadow2, lp.xyz, uniforms.uShadowTexel.z);
  } else if (d <= uniforms.uShadowSplits.w && uniforms.uShadowInfo.x > 3.5) {
    let p = world + nrm * (uniforms.uShadowParams.y * uniforms.uShadowWorld.w);
    let lp = uniforms.uShadowVP3 * vec4f(p, 1.0);
    res = shadowPcf(shadow3, lp.xyz, uniforms.uShadowTexel.w);
  }
  return res;
}
`;
