/**
 * Foliage mesh shader, WGSL (WebGPU path); twin of grass-material.glsl.ts (see there for the uniform contract).
 */
export const FOLIAGE_VERTEX_WGSL = /* wgsl */ `
attribute position: vec3f;
attribute normal: vec3f;
attribute color: vec4f;
attribute world0: vec4f;
attribute world1: vec4f;
attribute world2: vec4f;
attribute world3: vec4f;

uniform viewProjection: mat4x4f;
uniform uCam: vec3f;
uniform uWind: vec4f;
uniform uFade: vec2f;
uniform uParams: vec2f;

varying vNormal: vec3f;
varying vColor: vec3f;

@vertex
fn main(input: VertexInputs) -> FragmentInputs {
  let W = mat4x4f(vertexInputs.world0, vertexInputs.world1, vertexInputs.world2, vertexInputs.world3);
  var wp = (W * vec4f(vertexInputs.position, 1.0)).xyz;
  let base = vertexInputs.world3.xyz;
  let s = length(vertexInputs.world1.xyz);
  let phase = fract(sin(dot(base.xz, vec2f(12.9898, 78.233))) * 43758.5453) * 6.2831853;

  let sway = uniforms.uWind.z * vertexInputs.color.a * s * sin(uniforms.uWind.w * uniforms.uParams.y + phase);
  wp = vec3f(wp.x + uniforms.uWind.x * sway, wp.y, wp.z + uniforms.uWind.y * sway);

  if (uniforms.uFade.y > uniforms.uFade.x) {
    let k = 1.0 - smoothstep(uniforms.uFade.x, uniforms.uFade.y, distance(base, uniforms.uCam));
    wp = base + (wp - base) * k;
  }

  let n = normalize((W * vec4f(vertexInputs.normal, 0.0)).xyz);
  vertexOutputs.vNormal = normalize(mix(n, vec3f(0.0, 1.0, 0.0), uniforms.uParams.x));
  let tint = 0.9 + 0.2 * fract(phase * 7.31);
  vertexOutputs.vColor = vertexInputs.color.rgb * tint;
  vertexOutputs.position = uniforms.viewProjection * vec4f(wp, 1.0);
}
`;

export const FOLIAGE_FRAGMENT_WGSL = /* wgsl */ `
varying vNormal: vec3f;
varying vColor: vec3f;

@fragment
fn main(input: FragmentInputs) -> FragmentOutputs {
  let n = normalize(fragmentInputs.vNormal);
  let sun = normalize(vec3f(-0.45, 0.80, -0.35));
  let diff = max(dot(n, sun), 0.0);
  fragmentOutputs.color = vec4f(fragmentInputs.vColor * (0.34 + 0.74 * diff), 1.0);
}
`;
