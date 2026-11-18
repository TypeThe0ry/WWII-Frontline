/**
 * Engine-side PBR materials (M1-042b): `MaterialFactory` creates one `ShaderMaterial` per variant from the macro-expanded
 * sources in pbr-shader.ts, with the uniform contract the renderer sets once per frame (`setLighting`).
 */
import { Color3 } from '@babylonjs/core/Maths/math.color';
import { Vector3, Vector4 } from '@babylonjs/core/Maths/math.vector';
import { ShaderLanguage } from '@babylonjs/core/Materials/shaderLanguage';
import { ShaderMaterial } from '@babylonjs/core/Materials/shaderMaterial';
import type { Texture } from '@babylonjs/core/Materials/Textures/texture';
import type { Scene } from '@babylonjs/core/scene';
import { pbrGlsl, pbrWgsl } from './pbr-shader';
import { MaterialFactory, type MaterialDesc } from './variants';

export interface Lighting {
  /** Direction the light travels (unit). */
  dir: Vector3;
  color: Color3;
  ambient: Color3;
}

export const DEFAULT_LIGHTING: Lighting = { dir: new Vector3(-0.45, -0.8, -0.35).normalize(), color: new Color3(3.0, 2.9, 2.7), ambient: new Color3(0.35, 0.4, 0.5) };

export function createPbrMaterial(scene: Scene, d: MaterialDesc, key: string, wgsl: boolean): ShaderMaterial {
  const src = wgsl ? pbrWgsl(d) : pbrGlsl(d);
  const m = new ShaderMaterial(
    `${d.placeholderId ? `PH_${d.placeholderId}__` : 'PBR_'}${key.replace(':', '_')}`,
    scene,
    { vertexSource: src.vertex, fragmentSource: src.fragment },
    { attributes: src.attributes, uniforms: src.uniforms, samplers: src.samplers, shaderLanguage: wgsl ? ShaderLanguage.WGSL : ShaderLanguage.GLSL },
  );
  m.backFaceCulling = d.pass !== 'alpha';
  m.needDepthPrePass = false;
  if (d.pass === 'alpha') {
    m.alphaMode = 2; // ALPHA_COMBINE
    m.needAlphaBlending = () => true;
  }
  m.setVector4('uAlbedo', new Vector4(1, 1, 1, 1));
  m.setVector4('uPbr', new Vector4(0, 0.6, 0.5, 0)); // metallic, roughness, alpha cutoff
  m.setVector4('uWind', new Vector4(1, 0, 1, 0));
  return m;
}

export const makePbrFactory = (scene: Scene, wgsl: boolean, low = false): MaterialFactory<ShaderMaterial> => new MaterialFactory((d, key) => createPbrMaterial(scene, d, key, wgsl), low);

/** Per-frame inputs: camera position and the light. Call for every material the factory made (or let the renderer do it). */
export function setLighting(m: ShaderMaterial, camPos: Vector3, l: Lighting): void {
  m.setVector3('uCam', camPos);
  m.setVector3('uLightDir', l.dir);
  m.setColor3('uLightColor', l.color);
  m.setColor3('uAmbient', l.ambient);
}

/** Surface parameters of one object using this material (sets the material's uniforms; one material = one look until instancing params land). */
export function setSurface(m: ShaderMaterial, albedo: Color3, metallic: number, roughness: number, alphaCut = 0.5): void {
  m.setVector4('uAlbedo', new Vector4(albedo.r, albedo.g, albedo.b, 1));
  m.setVector4('uPbr', new Vector4(metallic, roughness, alphaCut, 0));
}

export function setTextures(m: ShaderMaterial, tex: { albedo?: Texture; normal?: Texture; detail?: Texture }): void {
  if (tex.albedo) m.setTexture('albedoTex', tex.albedo);
  if (tex.normal) m.setTexture('normalTex', tex.normal);
  if (tex.detail) m.setTexture('detailTex', tex.detail);
}
