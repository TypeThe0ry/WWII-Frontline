/**
 * `?scene=pbr` (M1-042b test scene): a grid of spheres (metallic 0/1 x roughness) lit by the PBR materials, a normal-mapped and
 * an alpha-tested plane, and the variant prewarm. Fixed camera, nothing time-dependent, so the same page in WebGPU and WebGL2
 * can be compared pixel by pixel (WGSL/GLSL delta E). `?pbr_many=N` adds N boxes on two variants for the material-freeze
 * benchmark; `?pbr_freeze=1` freezes the materials.
 */
import { Color3 } from '@babylonjs/core/Maths/math.color';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import { FreeCamera } from '@babylonjs/core/Cameras/freeCamera';
import { RawTexture } from '@babylonjs/core/Materials/Textures/rawTexture';
import { Constants } from '@babylonjs/core/Engines/constants';
import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import type { Mesh } from '@babylonjs/core/Meshes/mesh';
import type { ShaderMaterial } from '@babylonjs/core/Materials/shaderMaterial';
import { Color4 } from '@babylonjs/core/Maths/math.color';
import type { RenderContext } from '../engine';
import { DEFAULT_LIGHTING, makePbrFactory, setLighting, setSurface, setTextures } from './pbr-material';
import { prewarmVariants, type PrewarmResult } from './prewarm';
import { type MaterialDesc } from './variants';

export interface PbrSceneHandle {
  prewarm: PrewarmResult;
  variantCount: number;
  materials: ShaderMaterial[];
}

function tex(ctx: RenderContext, px: number[], w: number, h: number): RawTexture {
  const t = RawTexture.CreateRGBATexture(new Uint8Array(px), w, h, ctx.scene, false, false, Constants.TEXTURE_BILINEAR_SAMPLINGMODE);
  t.wrapU = Constants.TEXTURE_WRAP_ADDRESSMODE;
  t.wrapV = Constants.TEXTURE_WRAP_ADDRESSMODE;
  return t;
}

export async function attachPbrScene(ctx: RenderContext, params: URLSearchParams): Promise<PbrSceneHandle> {
  const scene = ctx.scene;
  scene.clearColor = new Color4(0.12, 0.13, 0.16, 1);
  const wgsl = ctx.kind === 'webgpu';
  const factory = makePbrFactory(scene, wgsl, params.get('quality') === 'low');
  const cam = new FreeCamera('pbr_cam', new Vector3(0, 2.2, -9), scene);
  cam.setTarget(new Vector3(0, 0.6, 0));
  cam.minZ = 0.1;
  cam.maxZ = 100;
  scene.activeCamera = cam;
  // textures: white albedo, a bumpy normal map (a 4x4 checker of tilted normals) and a mid-grey detail
  const white = tex(ctx, [255, 255, 255, 255], 1, 1);
  
  const normal = tex(ctx, Array.from({ length: 16 }, (_, i) => ((Math.floor(i / 4) + (i % 4)) % 2 ? [150, 128, 215, 255] : [106, 128, 215, 255])).flat(), 4, 4);
  const detail = tex(ctx, [128, 128, 128, 255], 1, 1);
  const holes = tex(ctx, Array.from({ length: 16 }, (_, i) => ((i % 4 < 2) === (Math.floor(i / 4) < 2) ? [255, 255, 255, 255] : [255, 255, 255, 0])).flat(), 4, 4);

  const mk = (d: MaterialDesc, albedo: Color3, metal: number, rough: number, textures: Parameters<typeof setTextures>[1] = { albedo: white }): ShaderMaterial => {
    const m = factory.getMaterial(d);
    setSurface(m, albedo, metal, rough);
    setTextures(m, { albedo: white, ...textures });
    return m;
  };
  const opaque: MaterialDesc = { shading: 'pbr', pass: 'opaque', flags: {} };
  const probe = MeshBuilder.CreateSphere('probe', { diameter: 1, segments: 16 }, scene);
  probe.setEnabled(false);

  // sphere grid: 5 roughness steps x (dielectric, metal)
  const rough = [0.1, 0.3, 0.5, 0.7, 0.95];
  for (let mi = 0; mi < 2; mi++) {
    for (let ri = 0; ri < rough.length; ri++) {
      const s = MeshBuilder.CreateSphere(`PH_V-GEN-001__s${mi}_${ri}`, { diameter: 1.3, segments: 32 }, scene);
      s.position.set((ri - 2) * 1.7, mi * 1.5, 0);
      // one material per (metal, roughness) look: each gets its own ShaderMaterial clone of the opaque variant
      const m = factory.getMaterial(opaque).clone(`PH_V-GEN-001__m${mi}_${ri}`) as ShaderMaterial;
      setSurface(m, mi ? new Color3(0.95, 0.64, 0.54) : new Color3(0.8, 0.15, 0.1), mi, rough[ri]!);
      setTextures(m, { albedo: white });
      s.material = m;
    }
  }
  // lower row: normal-mapped plane, alpha-tested plane, wind-bent plane
  const plane = (name: string, x: number, mat: ShaderMaterial): Mesh => {
    const p = MeshBuilder.CreatePlane(name, { size: 1.8 }, scene);
    p.position.set(x, -1.6, 0);
    p.rotation.x = Math.PI / 2 - 0.6;
    p.material = mat;
    return p;
  };
  plane('PH_V-GEN-001__normal', -2.2, mk({ shading: 'pbr', pass: 'opaque', flags: { normalMap: true, detail: true } }, new Color3(0.5, 0.6, 0.35), 0, 0.5, { albedo: white, normal, detail }));
  plane('PH_V-GEN-001__alpha', 0, mk({ shading: 'pbr', pass: 'opaque', flags: { alphaTest: true } }, new Color3(0.85, 0.8, 0.3), 0, 0.6, { albedo: holes }));
  plane('PH_V-GEN-001__blend', 2.2, mk({ shading: 'pbr', pass: 'alpha', flags: {} }, new Color3(0.3, 0.5, 0.9), 0, 0.3, { albedo: white }));

  const prewarm = await prewarmVariants(factory, probe);
  const mats = scene.materials.filter((m): m is ShaderMaterial => m.getClassName() === 'ShaderMaterial' && m.name.includes('PBR_')) as ShaderMaterial[];

  // freeze benchmark: N boxes on two variants
  const many = Number(params.get('pbr_many') ?? 0) | 0;
  if (many > 0) {
    const a = mk(opaque, new Color3(0.6, 0.6, 0.6), 0, 0.5);
    const b = mk({ shading: 'pbr', pass: 'opaque', flags: { normalMap: true } }, new Color3(0.5, 0.4, 0.3), 0, 0.6, { albedo: white, normal });
    for (let i = 0; i < many; i++) {
      const bx = MeshBuilder.CreateBox(`PH_V-GEN-001__many${i}`, { size: 0.3 }, scene);
      bx.position.set(((i % 50) - 25) * 0.6, 4 + Math.floor(i / 50) * 0.6, 6 + (i % 7) * 0.4);
      bx.material = i % 2 ? a : b;
      bx.freezeWorldMatrix();
    }
    if (params.get('pbr_freeze') === '1') {
      scene.freezeMaterials();
      scene.freezeActiveMeshes();
    }
  }

  // `?csm=custom`: a ground plane receives the spheres' shadows through the PBR shadow contract (PI-117)
  if (params.get('csm') === 'custom') {
    const { attachCustomCsm } = await import('../lighting/custom-csm');
    const ground = MeshBuilder.CreateGround('PH_V-GEN-001__ground', { width: 40, height: 40 }, scene);
    ground.position.y = -2.4;
    ground.material = mk({ shading: 'pbr', pass: 'opaque', flags: {} }, new Color3(0.55, 0.55, 0.5), 0, 0.8);
    const csm = attachCustomCsm(ctx, { tier: 'high', lightDir: [0.45, -0.8, 0.35], webgl2Config: params.get('csm_cfg') === 'webgl2' || undefined });
    for (const m of scene.meshes) if (m.name.startsWith('PH_V-GEN-001__s')) csm.addCaster(m);
    for (const m of scene.materials) if (m.getClassName() === 'ShaderMaterial' && m.name.includes('PBR_') || m.name.startsWith('PH_V-GEN-001__m')) csm.attach(m as ShaderMaterial);
    ctx.onFrame(() => csm.update(cam), 'first', 'csm_update');
    (window as unknown as { __csm?: typeof csm }).__csm = csm;
  }

  ctx.onFrame(() => {
    for (const m of scene.materials) if (m.getClassName() === 'ShaderMaterial') setLighting(m as ShaderMaterial, cam.position, DEFAULT_LIGHTING);
  }, 'first', 'material_update');
  return { prewarm, variantCount: factory.registry.stats().count, materials: mats };
}
