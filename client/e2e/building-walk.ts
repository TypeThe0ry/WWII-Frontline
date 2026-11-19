import { loadWasm } from '../src/game/net-game';
import type { WasmPredictor } from '../src/net/wasm';
import { LAYOUT_STATE_X, LAYOUT_STATE_Y, LAYOUT_STATE_Z } from '../src/net/wasm/layout';
import type { BuildingInterior } from '../src/render/world/building-interiors';
import { loadTerrainData } from '../src/render/terrain/terrain-data';

interface InteriorPredictor extends WasmPredictor {
  loadBuildingCollision(data: Uint8Array): void;
}

/** Run the exported sim KCC against the exact geometry already drawn by the Carentan world preview. */
export async function walkDoor(b: Pick<BuildingInterior, 'door' | 'base_y' | 'id'>) {
  const wasm = await loadWasm(import.meta.env.BASE_URL);
  const base = '/maps/carentan/';
  const [terrain, roomResponse] = await Promise.all([loadTerrainData(base), fetch(`${base}building-interiors.json`)]);
  if (!roomResponse.ok) throw new Error(`Building fixture returned ${roomResponse.status}`);
  const roomBytes = new Uint8Array(await roomResponse.arrayBuffer());
  const p = new wasm.Predictor(1n, 1, 1) as InteriorPredictor;
  try {
    const { samples, hmin_m, hmax_m } = terrain.meta.height;
    const [x, z] = b.door.center;
    const [nx, nz] = b.door.outward;
    const cx = Math.floor(x / 64), cz = Math.floor(z / 64);
    // The delivery pack uses FCOL; slice the identical height samples to avoid needing a full FPAK bake.
    for (let bz = cz - 1; bz <= cz + 1; bz++) {
      for (let bx = cx - 1; bx <= cx + 1; bx++) {
        const data = new Uint8Array(16 + 65 * 65 * 2 + 64 * 64);
        const v = new DataView(data.buffer);
        data.set([70, 67, 79, 76, 1]);
        v.setUint16(6, 65, true);
        v.setFloat32(8, hmin_m, true);
        v.setFloat32(12, hmax_m, true);
        for (let row = 0; row < 65; row++) {
          const offset = ((bz * 64 + row) * samples + bx * 64) * 2;
          data.set(terrain.nearBytes.subarray(offset, offset + 130), 16 + row * 130);
        }
        p.loadCollision(bx, bz, data);
      }
    }
    p.loadBuildingCollision(roomBytes);
    const auth = new DataView(new ArrayBuffer(80));
    let t = 0;
    const reset = (along: number, side: number) => {
      const px = x + nx * along - nz * side, pz = z + nz * along + nx * side;
      auth.setFloat32(0, px, true);
      auth.setFloat32(4, Math.max(terrain.heightAt(px, pz), along < 0 ? b.base_y : -Infinity) + 0.04, true);
      auth.setFloat32(8, pz, true);
      t = 0;
      p.reset(new Uint8Array(auth.buffer), 0);
    };
    const moving = (sign: number, cross: (d: number) => boolean) => {
      const yaw = Math.round(Math.atan2(sign * nx, sign * nz) * 65536 / (2 * Math.PI)) & 65535;
      for (let i = 0; i < 120; i++) {
        const s = p.step(wasm.packInputCmd(t, ++t, 0, 127, yaw, 0, 0, 0, 0, 0));
        const px = s[LAYOUT_STATE_X]!, py = s[LAYOUT_STATE_Y]!, pz = s[LAYOUT_STATE_Z]!;
        const distance = (px - x) * nx + (pz - z) * nz;
        if (cross(distance) || i === 119) return { distance, eye: [px, py + 1.55, pz] as [number, number, number] };
      }
      throw new Error('Missing KCC steps');
    };
    reset(1.4, 0);
    const entered = moving(-1, (d) => d < -0.8);
    reset(-0.95, 0);
    const exited = moving(1, (d) => d > 0.8);
    reset(1.05, 1.05);
    const blocked = moving(-1, () => false);
    return { entered, exited, blocked };
  } finally {
    p.free();
  }
}
