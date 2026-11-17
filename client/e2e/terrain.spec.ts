import { mkdirSync } from 'node:fs';
import { expect, test, type Page } from '@playwright/test';

// Software-rendered Chromium (SwiftShader): WebGL2 path is pixel-checked; the WebGPU path only checks that the WGSL
// compiles and draws without validation errors (the shim turns presenting into an offscreen texture; GPU times are H-021).
const SHIM = 'e2e/webgpu-swiftshader-shim.js';
const SHOT_DIR = '../docs/measurements/M0-023';
const SAVE_SHOTS = process.env.SAVE_SHOTS === '1';

// SwiftShader with 4x MSAA is ~10x slower at 960x540 than at 480x270; the checks below do not need the resolution.
test.use({ viewport: { width: 480, height: 270 } });

function watchErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}`));
  page.on('console', (m) => {
    if (m.type() === 'error') errors.push(`console.error: ${m.text()}`);
  });
  return errors;
}

async function ready(page: Page, minFrames = 3): Promise<void> {
  await page.waitForFunction(() => window.__terrain !== undefined, undefined, { timeout: 30_000 });
  await frames(page, minFrames);
}

async function frames(page: Page, n: number): Promise<void> {
  const f0 = await page.evaluate(() => window.__perf?.frames() ?? 0);
  await page.waitForFunction((k) => (window.__perf?.frames() ?? 0) >= k, f0 + n, { timeout: 60_000 });
}

type Vec3 = [number, number, number];
async function pose(page: Page, pos: Vec3, target: Vec3): Promise<void> {
  await page.evaluate(([p, t]) => window.__terrain!.setPose(p, t), [pos, target] as [Vec3, Vec3]);
  await frames(page, 2);
}

/** Decode the page screenshot into RGB pixels once; return a sampler plus a magenta counter. */
async function grab(page: Page, withData = false) {
  const png = (await page.screenshot()).toString('base64');
  return page.evaluate(async ([b64, keep]) => {
    const img = new Image();
    img.src = `data:image/png;base64,${b64}`;
    await img.decode();
    const c = document.createElement('canvas');
    c.width = img.width;
    c.height = img.height;
    const g = c.getContext('2d')!;
    g.drawImage(img, 0, 0);
    const d = g.getImageData(0, 0, img.width, img.height).data;
    let magenta = 0;
    let nonBackground = 0;
    for (let i = 0; i < d.length; i += 4) {
      if (d[i]! > 200 && d[i + 1]! < 110 && d[i + 2]! > 200) magenta++;
      else nonBackground++;
    }
    return { w: img.width, h: img.height, magenta, nonBackground, data: keep ? Array.from(d) : ([] as number[]) };
  }, [png, withData] as [string, boolean]);
}

function rng(seed: number): () => number {
  let s = seed >>> 0;
  return () => {
    s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
    return s / 4294967296;
  };
}

test('WebGL2: terrain draws as ONE instanced draw call; every debug mode compiles and renders (acceptance 5)', async ({ page }, info) => {
  const errors = watchErrors(page);
  await page.goto('/?renderer=webgl2&scene=terrain&debug=1');
  await ready(page, 5);
  const caps = await page.evaluate(() => window.__caps as { kind: string; fallbackReason: string | null });
  expect(caps.kind).toBe('webgl2');
  const dump = await page.evaluate(() => window.__perf!.dump());
  expect(dump.draw_calls).toBe(1);
  const st = await page.evaluate(() => window.__terrain!.stats() as { nodes: number; leaves: number; perLod: number[]; textureBytes: number; firstUpdateMs: number });
  expect(st.nodes).toBeGreaterThan(5);
  expect(st.nodes * 8192).toBeGreaterThan(0);
  expect(st.textureBytes).toBeLessThan(4 * 1024 * 1024);
  const loadMs = await page.evaluate(() => window.__terrain!.loadMs);
  console.log(`terrain load (fetch + decode + build tree + first select): ${loadMs.toFixed(0)} ms; first update ${st.firstUpdateMs.toFixed(1)} ms; texture upload ${(st.textureBytes / 1048576).toFixed(2)} MB; draw calls ${dump.draw_calls}; nodes ${st.nodes} (cut ${st.leaves}) per LOD ${st.perLod.join('/')}`);
  expect(loadMs).toBeLessThanOrEqual(500);
  for (const mode of ['none', 'lod', 'morph', 'normal', 'height', 'flat']) {
    await page.evaluate((m) => window.__terrain!.setDebug(m), mode);
    await frames(page, 3);
    const g = await grab(page);
    expect(g.nonBackground, `mode ${mode} produced pixels`).toBeGreaterThan(g.w * g.h * 0.2);
    await page.screenshot({ path: info.outputPath(`webgl2-${mode}.png`) });
  }
  expect(errors).toEqual([]);
});

test('GPU-sampled heights match the CPU surface (height debug mode, top-down, near block and far ring)', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/?renderer=webgl2&scene=terrain&terrain_debug=height');
  await ready(page);
  const rand = rng(21);
  const hmin = 1;
  const hmax = 11;
  // Poses chosen so the probed ground is in the finest LOD the data supports: the vertex grid then lies on the map
  // texels and the comparison tests sampling/dequantisation/orientation rather than LOD approximation.
  const cases: { label: string; poses: Vec3[]; half: [number, number]; tol: number; slope: number; outside: boolean }[] = [
    {
      label: 'near block (LOD0, 1 m grid)',
      poses: [[50, 50, 50], [128, 50, 50], [206, 50, 50], [50, 50, 128], [206, 50, 128], [50, 50, 206], [128, 50, 206], [206, 50, 206], [128, 50, 150]],
      half: [38, 20],
      tol: 0.1,
      slope: 0.12,
      outside: false,
    },
    { label: 'far ring (LOD3, 8 m grid = far texels)', poses: [[1000, 700, 500], [-1200, 700, 300], [300, 700, -900]], half: [150, 120], tol: 0.2, slope: 0.1, outside: true },
  ];
  for (const c of cases) {
    let worst = 0;
    let sum = 0;
    let used = 0;
    for (const p of c.poses) {
      await pose(page, p, [p[0], 0, p[2] + 0.4]);
      await frames(page, 1);
      const g = await grab(page, true);
      const cand = Array.from({ length: 120 }, (): [number, number] => [p[0] + (rand() * 2 - 1) * c.half[0], p[2] + (rand() * 2 - 1) * c.half[1]]);
      const probes = await page.evaluate(
        ([pts, maxSlope, outside]) => {
          const t = window.__terrain!;
          const out: { sx: number; sy: number; h: number }[] = [];
          for (const [x, z] of pts as [number, number][]) {
            if (outside && x > -20 && x < 276 && z > -20 && z < 276) continue;
            if (!outside && (x < 0 || x > 256 || z < 0 || z > 256)) continue;
            const gx = Math.abs(t.heightAt(x + 1, z) - t.heightAt(x - 1, z)) / 2;
            const gz = Math.abs(t.heightAt(x, z + 1) - t.heightAt(x, z - 1)) / 2;
            if (Math.max(gx, gz) > (maxSlope as number)) continue;
            const s = t.project(x, z);
            out.push({ sx: s.x, sy: s.y, h: t.heightAt(x, z) });
          }
          return out;
        },
        [cand, c.slope, c.outside] as const,
      );
      for (const q of probes) {
        const px = Math.round(q.sx);
        const py = Math.round(q.sy);
        if (px < 12 || py < 12 || px >= g.w - 12 || py >= g.h - 12) continue; // inside the canvas (it sits in the 8 px body margin)
        const rendered = hmin + (g.data[(py * g.w + px) * 4]! / 255) * (hmax - hmin);
        const err = Math.abs(rendered - q.h);
        worst = Math.max(worst, err);
        sum += err;
        used++;
      }
    }
    console.log(`${c.label}: ${used} probes on gentle ground, GPU-vs-CPU height error max ${worst.toFixed(3)} m, mean ${(sum / used).toFixed(3)} m (8-bit gray = ${((hmax - hmin) / 255).toFixed(3)} m/step)`);
    expect(used).toBeGreaterThan(40);
    expect(worst).toBeLessThanOrEqual(c.tol);
  }
  expect(errors).toEqual([]);
});

test('no cracks: flat shading over a magenta clear colour, random poses; control without edge masks does crack', async ({ page }) => {
  test.setTimeout(240_000);
  const errors = watchErrors(page);
  await page.goto('/?renderer=webgl2&scene=terrain&terrain_debug=flat');
  await ready(page);
  const rand = rng(77);
  const poses: [Vec3, Vec3][] = [];
  for (let i = 0; i < 14; i++) {
    const x = rand() * 256;
    const z = rand() * 256;
    const yaw = rand() * Math.PI * 2;
    const y = 2 + rand() * 25;
    poses.push([[x, y, z], [x + Math.sin(yaw) * 200, 6, z + Math.cos(yaw) * 200]]);
  }
  // Low grazing views from outside the block toward the far field and uphill (the geometry where a crack is see-through).
  for (let i = 0; i < 6; i++) {
    const yaw = rand() * Math.PI * 2;
    poses.push([[-300 + rand() * 900, 1.7 + rand() * 6, -300 + rand() * 900], [128 + Math.sin(yaw) * 900, 4, 128 + Math.cos(yaw) * 900]]);
  }
  let stitched = 0;
  let control = 0;
  for (const [p, t] of poses) {
    await page.evaluate(() => window.__terrain!.setStitch(true));
    await pose(page, p, t);
    stitched += (await grab(page)).magenta;
    await page.evaluate(() => window.__terrain!.setStitch(false));
    await pose(page, p, t);
    control += (await grab(page)).magenta;
  }
  console.log(`magenta (hole) pixels over ${poses.length} poses: stitched ${stitched}, control (no edge masks) ${control}`);
  expect(stitched).toBe(0);
  expect(control, 'detector sensitivity: the un-stitched control must show holes').toBeGreaterThan(0);
  expect(errors).toEqual([]);
});

test('far-ring edge is visible 4 km out and is a clean silhouette (acceptance 7, screenshot)', async ({ page }, info) => {
  const errors = watchErrors(page);
  await page.goto('/?renderer=webgl2&scene=terrain&terrain_debug=flat');
  await ready(page);
  await pose(page, [128, 12, 128], [-3000, 12, 128]);
  const g = await grab(page);
  // Scan the middle column from the bottom: terrain (white) must end at the horizon with background above it.
  const x = g.w >> 1;
  let edge = -1;
  for (let y = g.h - 1; y > 0; y--) {
    const i = (y * g.w + x) * 4;
    if (g.data[i]! > 200 && g.data[i + 1]! < 110) {
      edge = y;
      break;
    }
  }
  expect(edge, 'background (magenta) found above the terrain silhouette').toBeGreaterThan(0);
  expect(edge).toBeLessThan(g.h * 0.75);
  console.log(`far ring silhouette row ${edge}/${g.h}`);
  await page.screenshot({ path: info.outputPath('far-edge-flat.png') });
  expect(errors).toEqual([]);
});

test('WebGPU path (SwiftShader): WGSL terrain compiles and draws without validation errors (pixels not checked)', async ({ page }) => {
  const errors = watchErrors(page);
  await page.addInitScript({ path: SHIM });
  await page.goto('/?renderer=webgpu&scene=terrain&debug=1');
  await page.waitForFunction(() => window.__caps !== undefined, undefined, { timeout: 15_000 });
  const caps = await page.evaluate(() => window.__caps as { kind: string; fallbackReason: string | null });
  test.skip(caps.kind !== 'webgpu', `no WebGPU adapter in this browser (${caps.fallbackReason})`);
  await ready(page, 20);
  const st = await page.evaluate(() => window.__terrain!.stats() as { nodes: number });
  expect(st.nodes).toBeGreaterThan(5);
  for (const mode of ['lod', 'morph', 'normal', 'none']) {
    await page.evaluate((m) => window.__terrain!.setDebug(m), mode);
    await frames(page, 5);
  }
  const d = await page.evaluate(() => window.__perf!.dump());
  expect(d.draw_calls).toBe(1);
  expect(errors).toEqual([]);
});

test('reference screenshots: orbit around the block in lod/shaded modes (SAVE_SHOTS=1 writes docs/measurements/M0-023/)', async ({ page }) => {
  test.skip(!SAVE_SHOTS, 'set SAVE_SHOTS=1 to refresh the reference screenshots');
  test.setTimeout(240_000);
  await page.setViewportSize({ width: 960, height: 540 });
  mkdirSync(SHOT_DIR, { recursive: true });
  const errors = watchErrors(page);
  await page.goto('/?renderer=webgl2&scene=terrain&terrain_debug=lod');
  await ready(page);
  for (let i = 0; i < 8; i++) {
    const a = (i / 8) * Math.PI * 2;
    await pose(page, [128 + Math.sin(a) * 150, 45, 128 + Math.cos(a) * 150], [128, 4, 128]);
    await page.screenshot({ path: `${SHOT_DIR}/orbit-lod-${i}.png` });
  }
  await page.evaluate(() => window.__terrain!.setDebug('none'));
  await pose(page, [128, 12, 40], [128, 5, 200]);
  await page.screenshot({ path: `${SHOT_DIR}/ground-shaded.png` });
  await pose(page, [128, 14, 128], [-3000, 14, 128]);
  await page.screenshot({ path: `${SHOT_DIR}/far-ring-edge.png` });
  await page.evaluate(() => window.__terrain!.setDebug('normal'));
  await pose(page, [128, 110, -90], [128, 3, 140]);
  await page.screenshot({ path: `${SHOT_DIR}/overview-normal.png` });
  expect(errors).toEqual([]);
});
