import { Vector3, type Matrix } from '@babylonjs/core/Maths/math.vector';
import type { Scene } from '@babylonjs/core/scene';
import type { PlaceholderRegistry } from '../actors/names';
import type { FxManager } from '../fx';
import { TankModel, type TankDef, type TankState } from './tank-model';

/** Render-ready tank pose supplied by a sampler. Speeds are signed and angles are hull-relative for the turret and gun. */
export interface VehicleView extends TankState {
  id: number;
}

export class VehicleRenderer {
  private readonly models = new Map<number, TankModel>();
  private readonly shots: { id: number; own: boolean }[] = [];
  private readonly muzzle = new Vector3();

  constructor(
    private readonly scene: Scene,
    private readonly def: TankDef,
    private readonly registry: PlaceholderRegistry,
    private readonly fx: Pick<FxManager, 'emitTankShot'> | null = null,
  ) {}

  /** Queue an event before sync. Events for tanks absent from that frame are dropped. */
  queueShot(vehicleId: number, own = false): void {
    if (this.fx) this.shots.push({ id: vehicleId, own });
  }

  /** Sync the full set of visible tanks before rendering. `dtS` is used for track UV animation. */
  sync(views: readonly VehicleView[], dtS: number): void {
    const seen = new Set<number>();
    for (const view of views) {
      if (seen.has(view.id)) continue;
      seen.add(view.id);
      let model = this.models.get(view.id);
      if (!model) {
        model = new TankModel(this.scene, this.def, this.registry, view.id);
        this.models.set(view.id, model);
      }
      model.update(view, dtS, 'sampled');
    }
    for (const [id, model] of this.models) {
      if (seen.has(id)) continue;
      model.dispose();
      this.models.delete(id);
    }
    for (const shot of this.shots) {
      const socket = this.getSocket(shot.id, 'socket_gun_muzzle');
      if (!socket) continue;
      const { x, y, z } = socket.getTranslationToRef(this.muzzle);
      this.fx?.emitTankShot(x, y, z, shot.own);
    }
    this.shots.length = 0;
  }

  getModel(vehicleId: number): TankModel | undefined {
    return this.models.get(vehicleId);
  }

  /** Borrowed world transform, or null when the tank or socket is absent. Copy it to retain a snapshot. */
  getSocket(vehicleId: number, name: string): Matrix | null {
    return this.models.get(vehicleId)?.socketMatrix(name) ?? null;
  }

  dispose(): void {
    this.sync([], 0);
  }
}
