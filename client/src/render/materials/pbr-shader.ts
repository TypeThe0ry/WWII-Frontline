/**
 * PBR shader source (M1-042b, ARCHITECTURE 10.4): one template per language, four macros (`normalMap`, `alphaTest`,
 * `vertexWind`, `detail`) expanded into plain source text, so a variant is exactly the string this returns (no runtime branches).
 * Shading: Lambert + GGX specular (Smith-Schlick geometry, Schlick Fresnel) for one directional light plus a hemispheric
 * ambient term, then Reinhard-free linear output (the post chain tonemaps in M1-044b). The depth and shadow passes are
 * position-only (+ alpha test / wind). The GLSL and WGSL sources implement the same maths; `pbr-delta-e` checks them against
 * each other on the GPU.
 */
import { SHADOW_RECEIVER_GLSL, SHADOW_RECEIVER_WGSL, SHADOW_SAMPLERS, SHADOW_UNIFORMS } from '../lighting/csm-shader';
import type { MaterialDesc } from './variants';

export interface PbrSource {
  vertex: string;
  fragment: string;
  attributes: string[];
  uniforms: string[];
  samplers: string[];
}

const lit = (d: MaterialDesc): boolean => d.pass === 'opaque' || d.pass === 'alpha';

function interface_(d: MaterialDesc): Pick<PbrSource, 'attributes' | 'uniforms' | 'samplers'> {
  const attributes = ['position'];
  const uniforms = ['world', 'viewProjection', 'uWind'];
  const samplers: string[] = [];
  const f = d.flags;
  const needUv = lit(d) || f.alphaTest;
  if (lit(d)) attributes.push('normal');
  if (needUv) attributes.push('uv');
  if (lit(d)) uniforms.push('uCam', 'uAlbedo', 'uPbr', 'uLightDir', 'uLightColor', 'uAmbient', ...SHADOW_UNIFORMS);
  if (f.alphaTest && !lit(d)) uniforms.push('uPbr');
  if (needUv) samplers.push('albedoTex');
  if (f.normalMap && lit(d)) samplers.push('normalTex');
  if (f.detail && lit(d)) samplers.push('detailTex');
  if (lit(d)) samplers.push(...SHADOW_SAMPLERS); // cascade shadow maps (read with a texel fetch, no sampler object)
  return { attributes, uniforms, samplers };
}

export function pbrGlsl(d: MaterialDesc): PbrSource {
  const f = d.flags;
  const L = lit(d);
  const needUv = L || f.alphaTest === true;
  const vertex = `
precision highp float;
attribute vec3 position;
${L ? 'attribute vec3 normal;' : ''}
${needUv ? 'attribute vec2 uv;' : ''}
uniform mat4 world;
uniform mat4 viewProjection;
uniform vec4 uWind;
varying vec3 vPos;
${L ? 'varying vec3 vN;' : ''}
${needUv ? 'varying vec2 vUv;' : ''}
void main() {
  vec3 p = position;
  ${f.vertexWind ? 'p.xz += uWind.xy * uWind.z * p.y * 0.08 * sin(uWind.w + (world * vec4(p, 1.0)).x * 0.5);' : ''}
  vec4 wp = world * vec4(p, 1.0);
  vPos = wp.xyz;
  ${L ? 'vN = normalize(mat3(world) * normal);' : ''}
  ${needUv ? 'vUv = uv;' : ''}
  gl_Position = viewProjection * wp;
}
`;
  let fragment: string;
  if (L) {
    fragment = `
precision highp float;
varying vec3 vPos;
varying vec3 vN;
varying vec2 vUv;
uniform vec3 uCam;
uniform vec4 uAlbedo;
uniform vec4 uPbr; // metallic, roughness, alpha cutoff, 0
uniform vec3 uLightDir;
uniform vec3 uLightColor;
uniform vec3 uAmbient;
uniform sampler2D albedoTex;
${SHADOW_RECEIVER_GLSL}
${f.normalMap ? 'uniform sampler2D normalTex;' : ''}
${f.detail ? 'uniform sampler2D detailTex;' : ''}
const float PI = 3.14159265;
void main() {
  vec4 base = uAlbedo * texture2D(albedoTex, vUv);
  ${f.detail ? 'base.rgb *= texture2D(detailTex, vUv * 8.0).rgb * 2.0;' : ''}
  ${f.alphaTest ? 'if (base.a < uPbr.z) discard;' : ''}
  vec3 n = normalize(vN);
  ${f.normalMap ? `
  vec3 dp1 = dFdx(vPos); vec3 dp2 = dFdy(vPos); vec2 duv1 = dFdx(vUv); vec2 duv2 = dFdy(vUv);
  vec3 dp2perp = cross(dp2, n); vec3 dp1perp = cross(n, dp1);
  vec3 T = dp2perp * duv1.x + dp1perp * duv2.x; vec3 B = dp2perp * duv1.y + dp1perp * duv2.y;
  float inv = inversesqrt(max(max(dot(T, T), dot(B, B)), 1e-8));
  vec3 tn = texture2D(normalTex, vUv).xyz * 2.0 - 1.0;
  n = normalize(mat3(T * inv, B * inv, n) * tn);` : ''}
  vec3 v = normalize(uCam - vPos);
  vec3 l = normalize(-uLightDir);
  vec3 h = normalize(v + l);
  float nl = max(dot(n, l), 0.0);
  float nv = max(dot(n, v), 1e-4);
  float nh = max(dot(n, h), 0.0);
  float vh = max(dot(v, h), 0.0);
  float rough = clamp(uPbr.y, 0.04, 1.0);
  float a = rough * rough;
  float a2 = a * a;
  float dd = nh * nh * (a2 - 1.0) + 1.0;
  float D = a2 / (PI * dd * dd);
  float k = (rough + 1.0) * (rough + 1.0) / 8.0;
  float G = (nl / (nl * (1.0 - k) + k)) * (nv / (nv * (1.0 - k) + k));
  vec3 f0 = mix(vec3(0.04), base.rgb, uPbr.x);
  vec3 F = f0 + (1.0 - f0) * pow(1.0 - vh, 5.0);
  vec3 spec = D * G * F / max(4.0 * nl * nv, 1e-4);
  vec3 diff = (1.0 - F) * (1.0 - uPbr.x) * base.rgb / PI;
  vec3 amb = uAmbient * (0.5 + 0.5 * n.y) * mix(base.rgb, f0, uPbr.x);
  float sh = shadowFactor(vPos, n, uCam);
  gl_FragColor = vec4((diff + spec) * uLightColor * nl * sh + amb, ${d.pass === 'alpha' ? 'base.a' : '1.0'});
}
`;
  } else {
    fragment = `
precision highp float;
${f.alphaTest ? 'varying vec2 vUv;\nuniform sampler2D albedoTex;\nuniform vec4 uPbr;' : ''}
void main() {
  ${f.alphaTest ? 'if (texture2D(albedoTex, vUv).a < uPbr.z) discard;' : ''}
  gl_FragColor = vec4(gl_FragCoord.z, gl_FragCoord.z, gl_FragCoord.z, 1.0);
}
`;
  }
  return { vertex, fragment, ...interface_(d) };
}

export function pbrWgsl(d: MaterialDesc): PbrSource {
  const f = d.flags;
  const L = lit(d);
  const needUv = L || f.alphaTest === true;
  const vertex = `
attribute position: vec3f;
${L ? 'attribute normal: vec3f;' : ''}
${needUv ? 'attribute uv: vec2f;' : ''}
uniform world: mat4x4f;
uniform viewProjection: mat4x4f;
uniform uWind: vec4f;
varying vPos: vec3f;
${L ? 'varying vN: vec3f;' : ''}
${needUv ? 'varying vUv: vec2f;' : ''}
@vertex
fn main(input: VertexInputs) -> FragmentInputs {
  var p = vertexInputs.position;
  ${f.vertexWind ? 'p = vec3f(p.x + uniforms.uWind.x * uniforms.uWind.z * p.y * 0.08 * sin(uniforms.uWind.w + (uniforms.world * vec4f(p, 1.0)).x * 0.5), p.y, p.z + uniforms.uWind.y * uniforms.uWind.z * p.y * 0.08 * sin(uniforms.uWind.w + (uniforms.world * vec4f(p, 1.0)).x * 0.5));' : ''}
  let wp = uniforms.world * vec4f(p, 1.0);
  vertexOutputs.vPos = wp.xyz;
  ${L ? 'vertexOutputs.vN = normalize((uniforms.world * vec4f(vertexInputs.normal, 0.0)).xyz);' : ''}
  ${needUv ? 'vertexOutputs.vUv = vertexInputs.uv;' : ''}
  vertexOutputs.position = uniforms.viewProjection * wp;
}
`;
  let fragment: string;
  if (L) {
    fragment = `
varying vPos: vec3f;
varying vN: vec3f;
varying vUv: vec2f;
uniform uCam: vec3f;
uniform uAlbedo: vec4f;
uniform uPbr: vec4f;
uniform uLightDir: vec3f;
uniform uLightColor: vec3f;
uniform uAmbient: vec3f;
var albedoTex: texture_2d<f32>;
var albedoTexSampler: sampler;
${SHADOW_RECEIVER_WGSL}
${f.normalMap ? 'var normalTex: texture_2d<f32>;\nvar normalTexSampler: sampler;' : ''}
${f.detail ? 'var detailTex: texture_2d<f32>;\nvar detailTexSampler: sampler;' : ''}
const PI: f32 = 3.14159265;
@fragment
fn main(input: FragmentInputs) -> FragmentOutputs {
  let uvv = fragmentInputs.vUv;
  let wpos = fragmentInputs.vPos;
  var base = uniforms.uAlbedo * textureSample(albedoTex, albedoTexSampler, uvv);
  ${f.detail ? 'base = vec4f(base.rgb * textureSample(detailTex, detailTexSampler, uvv * 8.0).rgb * 2.0, base.a);' : ''}
  ${f.normalMap ? 'let tnS = textureSample(normalTex, normalTexSampler, uvv).xyz * 2.0 - 1.0;' : ''}
  let dp1 = dpdx(wpos); let dp2 = dpdy(wpos); let duv1 = dpdx(uvv); let duv2 = dpdy(uvv);
  ${f.alphaTest ? 'if (base.a < uniforms.uPbr.z) { discard; }' : ''}
  var n = normalize(fragmentInputs.vN);
  ${f.normalMap ? `
  let dp2perp = cross(dp2, n); let dp1perp = cross(n, dp1);
  let T = dp2perp * duv1.x + dp1perp * duv2.x; let B = dp2perp * duv1.y + dp1perp * duv2.y;
  let inv = inverseSqrt(max(max(dot(T, T), dot(B, B)), 1e-8));
  n = normalize(mat3x3f(T * inv, B * inv, n) * tnS);` : ''}
  let v = normalize(uniforms.uCam - wpos);
  let l = normalize(-uniforms.uLightDir);
  let h = normalize(v + l);
  let nl = max(dot(n, l), 0.0);
  let nv = max(dot(n, v), 1e-4);
  let nh = max(dot(n, h), 0.0);
  let vh = max(dot(v, h), 0.0);
  let rough = clamp(uniforms.uPbr.y, 0.04, 1.0);
  let a = rough * rough;
  let a2 = a * a;
  let dd = nh * nh * (a2 - 1.0) + 1.0;
  let D = a2 / (PI * dd * dd);
  let k = (rough + 1.0) * (rough + 1.0) / 8.0;
  let G = (nl / (nl * (1.0 - k) + k)) * (nv / (nv * (1.0 - k) + k));
  let f0 = mix(vec3f(0.04), base.rgb, uniforms.uPbr.x);
  let F = f0 + (vec3f(1.0) - f0) * pow(1.0 - vh, 5.0);
  let spec = D * G * F / max(4.0 * nl * nv, 1e-4);
  let diff = (vec3f(1.0) - F) * (1.0 - uniforms.uPbr.x) * base.rgb / PI;
  let amb = uniforms.uAmbient * (0.5 + 0.5 * n.y) * mix(base.rgb, f0, uniforms.uPbr.x);
  let sh = shadowFactor(wpos, n, uniforms.uCam);
  fragmentOutputs.color = vec4f((diff + spec) * uniforms.uLightColor * nl * sh + amb, ${d.pass === 'alpha' ? 'base.a' : '1.0'});
}
`;
  } else {
    fragment = `
${f.alphaTest ? 'varying vUv: vec2f;\nuniform uPbr: vec4f;\nvar albedoTex: texture_2d<f32>;\nvar albedoTexSampler: sampler;' : ''}
@fragment
fn main(input: FragmentInputs) -> FragmentOutputs {
  ${f.alphaTest ? 'let a = textureSample(albedoTex, albedoTexSampler, fragmentInputs.vUv).a;\n  if (a < uniforms.uPbr.z) { discard; }' : ''}
  let z = fragmentInputs.position.z;
  fragmentOutputs.color = vec4f(z, z, z, 1.0);
}
`;
  }
  return { vertex, fragment, ...interface_(d) };
}
