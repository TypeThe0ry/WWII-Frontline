import { Scene } from '@babylonjs/core/scene';
import type { AbstractEngine } from '@babylonjs/core/Engines/abstractEngine';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import { Color3, Color4 } from '@babylonjs/core/Maths/math.color';
import { MeshBuilder } from '@babylonjs/core/Meshes/meshBuilder';
import { StandardMaterial } from '@babylonjs/core/Materials/standardMaterial';
import { DirectionalLight } from '@babylonjs/core/Lights/directionalLight';
import { ArcRotateCamera } from '@babylonjs/core/Cameras/arcRotateCamera';
import { UniversalCamera } from '@babylonjs/core/Cameras/universalCamera';
import type { Camera } from '@babylonjs/core/Cameras/camera';

/** Clear colour of the test scene; e2e compares canvas samples against it. */
export const CLEAR_COLOR = { r: 0.04, g: 0.06, b: 0.1 } as const;

export interface TestSceneOptions {
  /** WASD + mouse free camera (reused by M0-023). */
  fly: boolean;
  /** Camera + light only (no meshes); noise-floor measurement for M0-030. */
  empty?: boolean;
}

function mat(scene: Scene, name: string, c: Color3): StandardMaterial {
  const m = new StandardMaterial(name, scene);
  m.diffuseColor = c;
  m.ambientColor = new Color3(0.35, 0.35, 0.35);
  m.specularColor = Color3.Black();
  return m;
}

/** Gray-box test scene: ground, boxes, spheres, grid lines, one directional light. No shadows/post. */
export function createTestScene(engine: AbstractEngine, canvas: HTMLCanvasElement, opts: TestSceneOptions): Scene {
  const scene = new Scene(engine);
  scene.clearColor = new Color4(CLEAR_COLOR.r, CLEAR_COLOR.g, CLEAR_COLOR.b, 1);
  scene.ambientColor = new Color3(1, 1, 1);

  const light = new DirectionalLight('PH_TEST_sun', new Vector3(-0.4, -1, -0.3), scene);
  light.intensity = 1.0;

  if (!opts.empty) addGrayBox(scene);
  return finishCamera(scene, canvas, opts);
}

function addGrayBox(scene: Scene): void {
  const ground = MeshBuilder.CreateGround('PH_TEST_ground', { width: 400, height: 400 }, scene);
  ground.material = mat(scene, 'PH_TEST_ground_mat', new Color3(0.25, 0.4, 0.22));

  // Grid lines every 10 m.
  const lines: Vector3[][] = [];
  for (let i = -50; i <= 50; i += 10) {
    lines.push([new Vector3(i, 0.02, -50), new Vector3(i, 0.02, 50)]);
    lines.push([new Vector3(-50, 0.02, i), new Vector3(50, 0.02, i)]);
  }
  const grid = MeshBuilder.CreateLineSystem('PH_TEST_grid', { lines }, scene);
  grid.color = new Color3(0.85, 0.85, 0.85);

  const boxMat = mat(scene, 'PH_TEST_box_mat', new Color3(0.75, 0.55, 0.3));
  const sphMat = mat(scene, 'PH_TEST_sphere_mat', new Color3(0.35, 0.5, 0.85));
  for (let i = 0; i < 8; i++) {
    const h = 1 + (i % 3);
    const b = MeshBuilder.CreateBox(`PH_TEST_box_${i}`, { width: 2, height: h, depth: 2 }, scene);
    b.position.set(-15 + i * 4, h / 2, 8 + (i % 2) * 4);
    b.material = boxMat;
  }
  for (let i = 0; i < 5; i++) {
    const s = MeshBuilder.CreateSphere(`PH_TEST_sphere_${i}`, { diameter: 2.5, segments: 16 }, scene);
    s.position.set(-8 + i * 5, 1.25, -6);
    s.material = sphMat;
  }

}

function finishCamera(scene: Scene, canvas: HTMLCanvasElement, opts: TestSceneOptions): Scene {
  let camera: Camera;
  if (opts.fly) {
    const cam = new UniversalCamera('PH_TEST_flycam', new Vector3(0, 3, -20), scene);
    cam.setTarget(new Vector3(0, 1.5, 0));
    cam.keysUp = [87];
    cam.keysDown = [83];
    cam.keysLeft = [65];
    cam.keysRight = [68];
    cam.speed = 0.4;
    cam.angularSensibility = 2500;
    cam.minZ = 0.1;
    cam.attachControl(canvas, true);
    canvas.addEventListener('click', () => {
      void canvas.requestPointerLock?.();
    });
    camera = cam;
  } else {
    const cam = new ArcRotateCamera('PH_TEST_orbitcam', -Math.PI / 2, 0.9, 45, new Vector3(0, 1, 0), scene);
    cam.minZ = 0.1;
    cam.wheelPrecision = 20;
    cam.attachControl(canvas, true);
    camera = cam;
  }
  scene.activeCamera = camera;
  return scene;
}
