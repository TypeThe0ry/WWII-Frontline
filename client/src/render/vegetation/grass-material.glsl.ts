/**
 * Foliage mesh shader, GLSL ES 3.00 (WebGL2 path): grass tufts, shrub and tree LOD meshes of M0-024. Reads the thin-instance
 * matrix attributes `world0..world3` (uniform scale, yaw about Y) and the placeholder vertex colour (a = sway weight).
 * grass-material.wgsl.ts is the WebGPU twin: keep the two in step (and wind.ts, which mirrors the sway formula).
 *
 * Uniforms: uCam camera position; uWind = (dirX, dirZ, strength m, time s); uFade = (start, end) metres, collapses
 * the instance onto its root between them (grass shrinks out instead of alpha-blending; end <= start disables);
 * uParams = (normal-up blend, sway omega rad/s).
 */
export const FOLIAGE_VERTEX_GLSL = /* glsl */ `
precision highp float;

attribute vec3 position;
attribute vec3 normal;
attribute vec4 color;
attribute vec4 world0;
attribute vec4 world1;
attribute vec4 world2;
attribute vec4 world3;

uniform mat4 viewProjection;
uniform vec3 uCam;
uniform vec4 uWind;
uniform vec2 uFade;
uniform vec2 uParams;

varying vec3 vNormal;
varying vec3 vColor;

void main() {
  mat4 W = mat4(world0, world1, world2, world3);
  vec3 wp = (W * vec4(position, 1.0)).xyz;
  vec3 base = world3.xyz;
  float s = length(world1.xyz);
  float phase = fract(sin(dot(base.xz, vec2(12.9898, 78.233))) * 43758.5453) * 6.2831853;

  float sway = uWind.z * color.a * s * sin(uWind.w * uParams.y + phase);
  wp.xz += uWind.xy * sway;

  if (uFade.y > uFade.x) {
    float k = 1.0 - smoothstep(uFade.x, uFade.y, distance(base, uCam));
    wp = base + (wp - base) * k;
  }

  vec3 n = normalize(mat3(W) * normal);
  vNormal = normalize(mix(n, vec3(0.0, 1.0, 0.0), uParams.x));
  float tint = 0.9 + 0.2 * fract(phase * 7.31);
  vColor = color.rgb * tint;
  gl_Position = viewProjection * vec4(wp, 1.0);
}
`;

export const FOLIAGE_FRAGMENT_GLSL = /* glsl */ `
precision highp float;

varying vec3 vNormal;
varying vec3 vColor;

void main() {
  vec3 n = normalize(vNormal);
  if (!gl_FrontFacing) n = -n;
  vec3 sun = normalize(vec3(-0.45, 0.80, -0.35));
  float diff = max(dot(n, sun), 0.0);
  gl_FragColor = vec4(vColor * (0.34 + 0.74 * diff), 1.0);
}
`;
