/**
 * `?scene=taa` (M1-044b ghosting test): a checkerboard ground and a bright box that moves a fixed step per loop frame, camera
 * static. `window.__taa.runTo(n)` lets the loop run to frame n (deterministic: the box position is a function of the frame index) and
 * stops it, so a screenshot afterwards can be compared with a reference rendered without TAA. Trail = where the box was in the last frames.
 */
import { Color3, Color4 } from '@babylonjs/core/Maths/math.color';
import { Matrix, Vector3 } from '@babylonjs/core/Maths/math.vector';
import { FreeCamera } from '@babylonjs/core/Cameras/freeCamera';
import { StandardMaterial } from '@babylonjs/core/Materials/standardMaterial';
import { RawTexture } from '@babylonjs/core/Materials/Textures/rawTexture';
import { Constants } from '@babylonjs/core/Engines/constants';
import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import type { RenderContext } from '../engine';

export interface TaaSceneHandle {
  /** Lets the render loop run until frame `n` has been rendered, then stops it; resolves with the box rectangle (page pixels) of that frame. */
  runTo(n: number): Promise<{ x: number; y: number; w: number; h: number }>;
  frame(): number;
}

export const BOX_STEP = 0.12; // world units per frame (about 3 px at the default camera)

export function attachTaaScene(ctx: RenderContext): TaaSceneHandle {
  const scene = ctx.scene;
  scene.clearColor = new Color4(0.1, 0.1, 0.12, 1);
  const cam = new FreeCamera('taa_cam', new Vector3(0, 3.5, -9), scene);
  cam.setTarget(new Vector3(0, 0.6, 0));
  cam.minZ = 0.1;
  scene.activeCamera = cam;
  const px: number[] = [];
  for (let y = 0; y < 16; y++) for (let x = 0; x < 16; x++) px.push(...((x + y) % 2 ? [70, 70, 78, 255] : [30, 30, 36, 255]));
  const tex = RawTexture.CreateRGBATexture(new Uint8Array(px), 16, 16, scene, false, false, Constants.TEXTURE_NEAREST_SAMPLINGMODE);
  tex.wrapU = Constants.TEXTURE_WRAP_ADDRESSMODE;
  tex.wrapV = Constants.TEXTURE_WRAP_ADDRESSMODE;
  const ground = MeshBuilder.CreateGround('PH_V-GEN-001__taa_ground', { width: 24, height: 12 }, scene);
  const gm = new StandardMaterial('PH_V-GEN-001__taa_ground_mat', scene);
  gm.emissiveTexture = tex;
  gm.disableLighting = true;
  (gm.emissiveTexture as RawTexture).uScale = 12;
  (gm.emissiveTexture as RawTexture).vScale = 6;
  ground.material = gm;
  const box = MeshBuilder.CreateBox('PH_V-GEN-001__taa_box', { width: 1, height: 1, depth: 1 }, scene);
  const bm = new StandardMaterial('PH_V-GEN-001__taa_box_mat', scene);
  bm.emissiveColor = new Color3(1, 0.95, 0.8);
  bm.disableLighting = true;
  box.material = bm;
  box.position.y = 0.5;
  let f = 0;
  const place = (): void => {
    box.position.x = -6 + ((f * BOX_STEP) % 12);
  };
  place();
  const eng = ctx.engine;
  let target = Infinity;
  let resolve: ((v: ReturnType<typeof boxRect>) => void) | null = null;
  const boxRect = (): { x: number; y: number; w: number; h: number } => {
    const vp = cam.viewport.toGlobal(eng.getRenderWidth(), eng.getRenderHeight());
    const tm = cam.getTransformationMatrix();
    const c = Vector3.Project(box.position.clone(), Matrix.IdentityReadOnly, tm, vp);
    const r = Vector3.Project(box.position.add(new Vector3(0.5, 0, 0)), Matrix.IdentityReadOnly, tm, vp);
    const rect = ctx.canvas.getBoundingClientRect();
    const sx = rect.width / eng.getRenderWidth();
    const sy = rect.height / eng.getRenderHeight();
    return { x: rect.left + c.x * sx, y: rect.top + c.y * sy, w: Math.abs(r.x - c.x) * 2 * sx, h: Math.abs(r.x - c.x) * 2 * sy };
  };
  // the box position is a function of the loop frame index; the loop is stopped right after frame `target` is rendered (post chain included)
  ctx.onFrame(() => {
    if (f >= target) return;
    f++;
    place();
  }, 'first', 'taa_scene');
  scene.onAfterRenderObservable.add(() => {
    if (f === target && resolve) {
      eng.stopRenderLoop();
      const r = resolve;
      resolve = null;
      r(boxRect());
    }
  });
  return {
    runTo(n) {
      return new Promise((res) => {
        target = n;
        resolve = res;
      });
    },
    frame: () => f,
  };
}
