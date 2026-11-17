/** Octahedral impostor shaders, WGSL (WebGPU); twins of impostor-material.glsl.ts (see there for the contract). */
export const IMPOSTOR_VERTEX_WGSL = /* wgsl */ `
attribute position: vec3f;
attribute world0: vec4f;
attribute world1: vec4f;
attribute world2: vec4f;
attribute world3: vec4f;

uniform viewProjection: mat4x4f;
uniform uCam: vec3f;
uniform uImp: vec4f;

varying vQ: vec2f;
varying vDirObj: vec3f;
varying vYaw: vec2f;

@vertex
fn main(input: VertexInputs) -> FragmentInputs {
  let base = vertexInputs.world3.xyz;
  let s = length(vertexInputs.world1.xyz);
  let cs = vertexInputs.world0.x / s;
  let sn = vertexInputs.world2.x / s;
  let C = base + vec3f(0.0, uniforms.uImp.y * s, 0.0);
  let d = normalize(uniforms.uCam - C);
  var r = cross(d, vec3f(0.0, 1.0, 0.0));
  let rl = length(r);
  if (rl > 1e-4) {
    r = r / rl;
  } else {
    r = vec3f(1.0, 0.0, 0.0);
  }
  let u = cross(r, d);
  let wp = C + (r * vertexInputs.position.x + u * vertexInputs.position.y) * (uniforms.uImp.x * s);
  vertexOutputs.vQ = vertexInputs.position.xy;
  vertexOutputs.vDirObj = vec3f(cs * d.x - sn * d.z, d.y, sn * d.x + cs * d.z);
  vertexOutputs.vYaw = vec2f(cs, sn);
  vertexOutputs.position = uniforms.viewProjection * vec4f(wp, 1.0);
}
`;

export const IMPOSTOR_FRAGMENT_WGSL = /* wgsl */ `
varying vQ: vec2f;
varying vDirObj: vec3f;
varying vYaw: vec2f;

uniform uImp: vec4f;
var uAtlasA: texture_2d<f32>;
var uAtlasASampler: sampler;
var uAtlasN: texture_2d<f32>;
var uAtlasNSampler: sampler;

@fragment
fn main(input: FragmentInputs) -> FragmentOutputs {
  var d = normalize(fragmentInputs.vDirObj);
  d.y = max(d.y, 0.0);
  let xz = vec2f(d.x, d.z) / (abs(d.x) + d.y + abs(d.z));
  let ab = vec2f(xz.x + xz.y, xz.x - xz.y);
  let cells = uniforms.uImp.z;
  let cell = clamp(floor((ab * 0.5 + 0.5) * cells), vec2f(0.0), vec2f(cells - 1.0));
  let q01 = clamp(fragmentInputs.vQ * 0.5 + 0.5, vec2f(0.004), vec2f(0.996));
  var uv = (cell + q01) / cells;
  if (uniforms.uImp.w > 0.5) {
    uv.y = 1.0 - uv.y;
  }
  let col = textureSample(uAtlasA, uAtlasASampler, uv);
  let nTex = textureSample(uAtlasN, uAtlasNSampler, uv).rgb;
  if (col.a < 0.5) {
    discard;
  }
  let nObj = nTex * 2.0 - 1.0;
  let c = fragmentInputs.vYaw.x;
  let sn = fragmentInputs.vYaw.y;
  let n = normalize(vec3f(c * nObj.x + sn * nObj.z, nObj.y, -sn * nObj.x + c * nObj.z));
  let sun = normalize(vec3f(-0.45, 0.80, -0.35));
  let diff = max(dot(n, sun), 0.0);
  fragmentOutputs.color = vec4f(col.rgb * (0.34 + 0.74 * diff), 1.0);
}
`;

export const BAKE_VERTEX_WGSL = /* wgsl */ `
attribute position: vec3f;
attribute normal: vec3f;
attribute color: vec4f;
attribute world0: vec4f;
attribute world1: vec4f;
attribute world2: vec4f;
attribute world3: vec4f;

uniform uImp: vec4f;
uniform uMode: f32;

varying vOut: vec3f;

@vertex
fn main(input: VertexInputs) -> FragmentInputs {
  let cells = uniforms.uImp.z;
  let idx = vertexInputs.world0.x;
  let ci = idx % cells;
  let cj = floor(idx / cells);
  let ab = (vec2f(ci, cj) + 0.5) / cells * 2.0 - 1.0;
  let x = (ab.x + ab.y) * 0.5;
  let z = (ab.x - ab.y) * 0.5;
  let d = normalize(vec3f(x, max(1.0 - abs(x) - abs(z), 0.0), z));
  var r = cross(d, vec3f(0.0, 1.0, 0.0));
  let rl = length(r);
  if (rl > 1e-4) {
    r = r / rl;
  } else {
    r = vec3f(1.0, 0.0, 0.0);
  }
  let u = cross(r, d);
  let p = vertexInputs.position - vec3f(0.0, uniforms.uImp.y, 0.0);
  let xy = vec2f(dot(p, r), dot(p, u)) / uniforms.uImp.x;
  let depth = dot(p, d) / uniforms.uImp.x;
  var zc = -depth * 0.98;
  if (uniforms.uImp.w > 0.5) {
    zc = 0.5 - depth * 0.49;
  }
  var o = vertexInputs.color.rgb;
  if (uniforms.uMode > 0.5) {
    o = normalize(vertexInputs.normal) * 0.5 + 0.5;
  }
  vertexOutputs.vOut = o;
  vertexOutputs.position = vec4f(ab + xy / cells, zc, 1.0);
}
`;

export const BAKE_FRAGMENT_WGSL = /* wgsl */ `
varying vOut: vec3f;

@fragment
fn main(input: FragmentInputs) -> FragmentOutputs {
  fragmentOutputs.color = vec4f(fragmentInputs.vOut, 1.0);
}
`;
