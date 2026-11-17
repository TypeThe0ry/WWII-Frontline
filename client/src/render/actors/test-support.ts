// Test helpers for the actor tests (vitest only; not imported by the app): a NullEngine scene, real data from the repo.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { NullEngine } from '@babylonjs/core/Engines/nullEngine';
import { UniversalCamera } from '@babylonjs/core/Cameras/universalCamera';
import { Vector3 } from '@babylonjs/core/Maths/math.vector';
import { Scene } from '@babylonjs/core/scene';
import { HAVE_WASM, loadWasm } from '../../net/test-support';
import { buildActorData, type ActorData, type DataSource, type WeaponModelsFile } from './actor-data';

export { HAVE_WASM, loadWasm };

const ROOT = fileURLToPath(new URL('../../../../', import.meta.url));
export const MODELS_FILE = `${ROOT}data/placeholders/weapon_models.json`;

export function readModels(): WeaponModelsFile {
  return JSON.parse(readFileSync(MODELS_FILE, 'utf8')) as WeaponModelsFile;
}

export async function realData(patchRole?: (r: { capsule: Record<string, number> }) => void): Promise<{ data: ActorData; src: DataSource }> {
  const wasm = await loadWasm();
  const src: DataSource = {
    roleDef: (id) => {
      const b = wasm.roleDef(id);
      if (!patchRole) return b;
      const j = JSON.parse(new TextDecoder().decode(b)) as { capsule: Record<string, number> };
      patchRole(j);
      return new TextEncoder().encode(JSON.stringify(j));
    },
    weaponDef: (id) => wasm.weaponDef(id),
    weaponIndex: (id) => wasm.weaponIndex(id),
  };
  return { data: buildActorData(src, readModels()), src };
}

export interface TestScene {
  engine: NullEngine;
  scene: Scene;
  camera: UniversalCamera;
  /** One engine frame: beginFrame, `body`, render, endFrame (what the real render loop does). */
  frame(body?: () => void): void;
  dispose(): void;
}

export function makeScene(): TestScene {
  const engine = new NullEngine();
  const scene = new Scene(engine);
  const camera = new UniversalCamera('PH_TEST_cam', new Vector3(0, 2, -5), scene);
  return {
    engine,
    scene,
    camera,
    frame(body) {
      engine.beginFrame();
      body?.();
      scene.render();
      engine.endFrame();
    },
    dispose() {
      scene.dispose();
      engine.dispose();
    },
  };
}
