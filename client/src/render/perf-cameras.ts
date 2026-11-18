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

/**
 * The three M1 camera scenes of the performance benchmark (M1-069, `tools/perf/m1-scenes.json`): soldier eye height on the
 * Carentan map (`?scene=world&map=carentan&cam=open_field|hedgerow|old_town`). `target.y = 0` / `pos.y = 0` resolve to
 * terrain height + eye height like the M0 poses. Poses were picked from the map data (densest 100 m disc of buildings;
 * the point farthest from any building or hedge; the longest hedge line that is not on the map edge). Changing any value
 * invalidates M1 baselines; the unit test pins them.
 */
export const M1_PERF_CAMERAS: readonly PerfCamera[] = [
  { name: 'open_field', pos: [700, 0, 1330], target: [700, 0, 1470] }, // 209 m from the nearest building or hedge
  { name: 'hedgerow', pos: [1500, 0, 500], target: [1300, 0, 540] }, // along the 180 m hedge line (1475,517)-(1297,537)
  { name: 'old_town', pos: [1246, 0, 700], target: [1275, 0, 880] }, // 175 buildings within 100 m; Place de la Republique and the church ahead
];

/** Heading of a pose as the sim yaw (0 = +z, growing toward +x). */
export const poseYaw = (c: PerfCamera): number => Math.atan2(c.target[0] - c.pos[0], c.target[2] - c.pos[2]);
