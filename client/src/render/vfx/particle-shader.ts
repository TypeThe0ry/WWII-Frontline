/**
 * Billboard particle shader (M1-047b), GLSL (WebGL2) and WGSL (WebGPU) twins. One quad per thin instance; the instance
 * matrix carries (size, kind, age01) in `world0.xyz` and the centre in `world3.xyz` (see `ParticlePool.writeInstances`).
 * Uniforms: uRight / uUp = camera basis. Colour and alpha come from the kind and the normalised age; no textures (placeholder
 * V-FX-001 look: soft-edged flat colours).
 */
export const PARTICLE_VERTEX_GLSL = /* glsl */ `
precision highp float;
attribute vec3 position;
attribute vec4 world0;
attribute vec4 world3;
uniform mat4 viewProjection;
uniform vec3 uRight;
uniform vec3 uUp;
varying vec2 vUv;
varying vec4 vColor;

vec3 kindColor(float k) {
  if (k < 0.5) return vec3(1.0, 0.82, 0.45);
  if (k < 1.5) return vec3(1.0, 0.95, 0.6);
  if (k < 2.5) return vec3(0.62, 0.62, 0.62);
  if (k < 3.5) return vec3(1.0, 0.5, 0.15);
  if (k < 4.5) return vec3(0.7, 0.6, 0.45);
  return vec3(0.8, 0.65, 0.2);
}

void main() {
  float age = world0.z;
  float grow = 1.0 + (world0.y > 1.5 && world0.y < 3.5 ? age * 1.5 : 0.0);
  vec3 wp = world3.xyz + (uRight * position.x + uUp * position.y) * world0.x * grow;
  vUv = position.xy * 2.0;
  vColor = vec4(kindColor(world0.y), (1.0 - age) * (world0.y < 0.5 ? 1.0 : 0.8));
  gl_Position = viewProjection * vec4(wp, 1.0);
}
`;

export const PARTICLE_FRAGMENT_GLSL = /* glsl */ `
precision highp float;
varying vec2 vUv;
varying vec4 vColor;
void main() {
  float d = dot(vUv, vUv);
  if (d > 1.0) discard;
  gl_FragColor = vec4(vColor.rgb, vColor.a * (1.0 - d));
}
`;

export const PARTICLE_VERTEX_WGSL = /* wgsl */ `
attribute position: vec3f;
attribute world0: vec4f;
attribute world3: vec4f;
uniform viewProjection: mat4x4f;
uniform uRight: vec3f;
uniform uUp: vec3f;
varying vUv: vec2f;
varying vColor: vec4f;

fn kindColor(k: f32) -> vec3f {
  if (k < 0.5) { return vec3f(1.0, 0.82, 0.45); }
  if (k < 1.5) { return vec3f(1.0, 0.95, 0.6); }
  if (k < 2.5) { return vec3f(0.62, 0.62, 0.62); }
  if (k < 3.5) { return vec3f(1.0, 0.5, 0.15); }
  if (k < 4.5) { return vec3f(0.7, 0.6, 0.45); }
  return vec3f(0.8, 0.65, 0.2);
}

@vertex
fn main(input: VertexInputs) -> FragmentInputs {
  let w0 = vertexInputs.world0;
  let age = w0.z;
  var grow = 1.0;
  if (w0.y > 1.5 && w0.y < 3.5) { grow = 1.0 + age * 1.5; }
  let wp = vertexInputs.world3.xyz + (uniforms.uRight * vertexInputs.position.x + uniforms.uUp * vertexInputs.position.y) * w0.x * grow;
  vertexOutputs.vUv = vertexInputs.position.xy * 2.0;
  var a = 0.8;
  if (w0.y < 0.5) { a = 1.0; }
  vertexOutputs.vColor = vec4f(kindColor(w0.y), (1.0 - age) * a);
  vertexOutputs.position = uniforms.viewProjection * vec4f(wp, 1.0);
}
`;

export const PARTICLE_FRAGMENT_WGSL = /* wgsl */ `
varying vUv: vec2f;
varying vColor: vec4f;
@fragment
fn main(input: FragmentInputs) -> FragmentOutputs {
  let d = dot(fragmentInputs.vUv, fragmentInputs.vUv);
  if (d > 1.0) { discard; }
  fragmentOutputs.color = vec4f(fragmentInputs.vColor.rgb, fragmentInputs.vColor.a * (1.0 - d));
}
`;
