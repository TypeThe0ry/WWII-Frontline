/**
 * The three fixed camera poses of the M0 performance baseline (M0-030) and of the terrain / vegetation debug presets:
 * `?scene=terrain|vegetation&cam=0|1|2`. `y = 0` means "terrain height + eye height" (1.7 m for the position, 1.5 m for the
 * target), resolved by the scene. They are part of the baseline definition: changing one invalidates
 * `reports/baselines/client-m0.json` (the unit test pins the numbers). Terrain time and vegetation seed are fixed too
 * (`veg_seed` defaults to 1; nothing in these scenes depends on wall-clock time).
 */
export interface PerfCamera {
  name: string;
  pos: [number, number, number];
  target: [number, number, number];
}

export const PERF_CAMERAS: readonly PerfCamera[] = [
  { name: 'ground-north', pos: [128, 0, 40], target: [128, 0, 128] }, // 0: ground level looking north (open terrain + near vegetation)
  { name: 'overview', pos: [128, 110, -90], target: [128, 0, 140] }, // 1: elevated overview of the whole block (most instances)
  { name: 'west-edge', pos: [20, 12, 128], target: [-3000, 0, 128] }, // 2: block edge looking west over the far ring (ridge view)
];
