/**
 * Octahedral impostor shaders, GLSL ES 3.00 (WebGL2). Two programs:
 *  - IMPOSTOR_*: the runtime camera-facing quad. Picks the atlas cell of the (object-space) camera direction with the
 *    hemi-octahedral mapping of impostor-bake.ts (`hemiOctEncode`), samples albedo + normal, alpha-tests, lights like
 *    the mesh LODs.
 *  - BAKE_*: draws all 12 x 12 views of a placeholder mesh into one atlas render target in a single instanced draw
 *    (instance attribute world0.x = cell index; orthographic basis built in the vertex shader), so the bake needs no
 *    per-view camera/viewport/clear handling and behaves the same on WebGL2 and WebGPU.
 * The WebGPU twins are in impostor-material.wgsl.ts. The basis (right = cross(d, up), camUp = cross(right, d)) must be
 * identical in IMPOSTOR_VERTEX and BAKE_VERTEX.
 *
 * uImp = (radius R, centre height cy, cells per side, flipV): flipV = 1 on WebGPU, where NDC y-up lands on texture
 * row 0 (v = 0) while WebGL has it at v = 1.
 */
export const IMPOSTOR_VERTEX_GLSL = /* glsl */ `
precision highp float;

attribute vec3 position;   // quad corner (-1..1, -1..1, 0)
attribute vec4 world0;
attribute vec4 world1;
attribute vec4 world2;
attribute vec4 world3;

uniform mat4 viewProjection;
uniform vec3 uCam;
uniform vec4 uImp;

varying vec2 vQ;
varying vec3 vDirObj;
varying vec2 vYaw;

void main() {
  vec3 base = world3.xyz;
  float s = length(world1.xyz);
  float cs = world0.x / s;
  float sn = world2.x / s;
  vec3 C = base + vec3(0.0, uImp.y * s, 0.0);
  vec3 d = normalize(uCam - C);
  vec3 r = cross(d, vec3(0.0, 1.0, 0.0));
  float rl = length(r);
  r = rl > 1e-4 ? r / rl : vec3(1.0, 0.0, 0.0);
  vec3 u = cross(r, d);
  vec3 wp = C + (r * position.x + u * position.y) * (uImp.x * s);
  vQ = position.xy;
  vDirObj = vec3(cs * d.x - sn * d.z, d.y, sn * d.x + cs * d.z);
  vYaw = vec2(cs, sn);
  gl_Position = viewProjection * vec4(wp, 1.0);
}
`;

export const IMPOSTOR_FRAGMENT_GLSL = /* glsl */ `
precision highp float;

varying vec2 vQ;
varying vec3 vDirObj;
varying vec2 vYaw;

uniform vec4 uImp;
uniform sampler2D uAtlasA;
uniform sampler2D uAtlasN;

void main() {
  vec3 d = normalize(vDirObj);
  d.y = max(d.y, 0.0);
  vec2 xz = d.xz / (abs(d.x) + d.y + abs(d.z));
  vec2 ab = vec2(xz.x + xz.y, xz.x - xz.y);
  float cells = uImp.z;
  vec2 cell = clamp(floor((ab * 0.5 + 0.5) * cells), vec2(0.0), vec2(cells - 1.0));
  vec2 q01 = clamp(vQ * 0.5 + 0.5, vec2(0.004), vec2(0.996));
  vec2 uv = (cell + q01) / cells;
  if (uImp.w > 0.5) uv.y = 1.0 - uv.y;
  vec4 col = texture2D(uAtlasA, uv);
  if (col.a < 0.5) discard;
  vec3 nObj = texture2D(uAtlasN, uv).rgb * 2.0 - 1.0;
  // object -> world: x' = c x + s z, z' = -s x + c z
  vec3 n = normalize(vec3(vYaw.x * nObj.x + vYaw.y * nObj.z, nObj.y, -vYaw.y * nObj.x + vYaw.x * nObj.z));
  vec3 sun = normalize(vec3(-0.45, 0.80, -0.35));
  float diff = max(dot(n, sun), 0.0);
  gl_FragColor = vec4(col.rgb * (0.34 + 0.74 * diff), 1.0);
}
`;

export const BAKE_VERTEX_GLSL = /* glsl */ `
precision highp float;

attribute vec3 position;
attribute vec3 normal;
attribute vec4 color;
attribute vec4 world0;
attribute vec4 world1;
attribute vec4 world2;
attribute vec4 world3;

uniform vec4 uImp;     // R, cy, cells, halfZ (1 when clip z is [0,1], i.e. WebGPU)
uniform float uMode;   // 0 albedo, 1 object-space normal

varying vec3 vOut;

void main() {
  float cells = uImp.z;
  float idx = world0.x;
  float ci = mod(idx, cells);
  float cj = floor(idx / cells);
  vec2 ab = (vec2(ci, cj) + 0.5) / cells * 2.0 - 1.0;
  float x = (ab.x + ab.y) * 0.5;
  float z = (ab.x - ab.y) * 0.5;
  vec3 d = normalize(vec3(x, max(1.0 - abs(x) - abs(z), 0.0), z));
  vec3 r = cross(d, vec3(0.0, 1.0, 0.0));
  float rl = length(r);
  r = rl > 1e-4 ? r / rl : vec3(1.0, 0.0, 0.0);
  vec3 u = cross(r, d);
  vec3 p = position - vec3(0.0, uImp.y, 0.0);
  vec2 xy = vec2(dot(p, r), dot(p, u)) / uImp.x;
  float depth = dot(p, d) / uImp.x;   // +1 = towards the viewer
  float zc = uImp.w > 0.5 ? 0.5 - depth * 0.49 : -depth * 0.98;
  vOut = uMode > 0.5 ? normalize(normal) * 0.5 + 0.5 : color.rgb;
  gl_Position = vec4(ab + xy / cells, zc, 1.0);
}
`;

export const BAKE_FRAGMENT_GLSL = /* glsl */ `
precision highp float;
varying vec3 vOut;
void main() {
  gl_FragColor = vec4(vOut, 1.0);
}
`;
