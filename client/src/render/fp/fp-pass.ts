/**
 * First-person weapon pass (M1-046b, ARCHITECTURE 10.3): the weapon is drawn by its own camera with its own FOV and its own
 * depth buffer clear, on top of the world. The world camera's FOV (settings 40-110 deg, ADS zoom) therefore never changes the
 * on-screen size of the weapon (`fp_fov_independent`), and the weapon never clips into geometry (the depth of the world pass is
 * cleared before it). Meshes opt in through `FP_LAYER`; the main camera's layer mask excludes it.
 */
import { FreeCamera } from '@babylonjs/core/Cameras/freeCamera';
import type { Camera } from '@babylonjs/core/Cameras/camera';
import type { TargetCamera } from '@babylonjs/core/Cameras/targetCamera';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import type { AbstractMesh } from '@babylonjs/core/Meshes/abstractMesh';
import type { Scene } from '@babylonjs/core/scene';

export const FP_LAYER = 0x10000000;
const ALL_BUT_FP = 0x0fffffff;

export class FpWeaponPass {
  readonly camera: FreeCamera;
  private readonly prevActive: Camera[] | null;

  constructor(
    private readonly scene: Scene,
    private readonly world: TargetCamera,
    /** Vertical FOV (rad) of the weapon camera; fixed, independent of the world camera. */
    readonly fovRad: number,
  ) {
    this.prevActive = scene.activeCameras ? [...scene.activeCameras] : null;
    world.layerMask = ALL_BUT_FP;
    this.camera = new FreeCamera('fp_weapon_camera', world.position.clone(), scene);
    this.camera.layerMask = FP_LAYER;
    this.camera.fov = fovRad;
    this.camera.minZ = 0.02;
    this.camera.maxZ = 10;
    this.camera.rotationQuaternion = null;
    this.camera.detachControl();
    scene.activeCameras = [world, this.camera];
    scene.activeCamera = world;
  }

  /** Marks a mesh (and its children) as belonging to the weapon pass. */
  adopt(root: AbstractMesh | { getChildMeshes(): AbstractMesh[] }, extra: readonly AbstractMesh[] = []): void {
    const list = [...extra, ...('getChildMeshes' in root ? root.getChildMeshes() : [])];
    for (const m of list) m.layerMask = FP_LAYER;
  }

  /** Copy the world camera pose (the weapon camera looks exactly where the player looks) and keep the FOV fixed. */
  sync(): void {
    const w = this.world;
    this.camera.position.copyFrom(w.position);
    this.camera.rotation.copyFrom(w.rotation as Vector3);
    this.camera.fov = this.fovRad;
    const eng = this.scene.getEngine();
    this.camera.getProjectionMatrix(true);
    void eng;
  }

  dispose(): void {
    this.scene.activeCameras = this.prevActive;
    this.world.layerMask = 0x0fffffff | FP_LAYER;
    this.camera.dispose();
  }
}
