# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: terrain.spec.ts >> WebGL2: terrain draws as ONE instanced draw call; every debug mode compiles and renders (acceptance 5)
- Location: e2e/terrain.spec.ts:69:1

# Error details

```
Test timeout of 60000ms exceeded.
```

```
Error: page.screenshot: Test timeout of 60000ms exceeded.
Call log:
  - taking page screenshot
  - waiting for fonts to load...
  - fonts loaded

```

# Page snapshot

```yaml
- generic [active] [ref=e1]: renderer WebGL2 FPS 4.4 CPU ms 0.79 P50 0.50 P99 5.00 (n=263) GPU ms 193.52 ms (disjoint-timer-query) draws 1 meshes 1 tris 0 res 480x270 render_scale 1.00 sim steps 642 clamped 103 RTT n/a transport n/a corrections n/a terrain nodes 38 (cut 76) tris 311296 select 0.100 ms per LOD 14 6 6 4 4 4 0 tex 2.13 MB load 93 ms
```

# Test source

```ts
  1   | import { mkdirSync } from 'node:fs';
  2   | import { expect, test, type Page } from '@playwright/test';
  3   | 
  4   | // Software-rendered Chromium (SwiftShader): WebGL2 path is pixel-checked; the WebGPU path only checks that the WGSL
  5   | // compiles and draws without validation errors (the shim turns presenting into an offscreen texture; GPU times are H-021).
  6   | const SHIM = 'e2e/webgpu-swiftshader-shim.js';
  7   | const SHOT_DIR = '../docs/measurements/M0-023';
  8   | const SAVE_SHOTS = process.env.SAVE_SHOTS === '1';
  9   | 
  10  | // SwiftShader with 4x MSAA is ~10x slower at 960x540 than at 480x270; the checks below do not need the resolution.
  11  | test.use({ viewport: { width: 480, height: 270 } });
  12  | 
  13  | function watchErrors(page: Page): string[] {
  14  |   const errors: string[] = [];
  15  |   page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}`));
  16  |   page.on('console', (m) => {
  17  |     if (m.type() === 'error') errors.push(`console.error: ${m.text()}`);
  18  |   });
  19  |   return errors;
  20  | }
  21  | 
  22  | async function ready(page: Page, minFrames = 3): Promise<void> {
  23  |   await page.waitForFunction(() => window.__terrain !== undefined, undefined, { timeout: 30_000 });
  24  |   await frames(page, minFrames);
  25  | }
  26  | 
  27  | async function frames(page: Page, n: number): Promise<void> {
  28  |   const f0 = await page.evaluate(() => window.__perf?.frames() ?? 0);
  29  |   await page.waitForFunction((k) => (window.__perf?.frames() ?? 0) >= k, f0 + n, { timeout: 60_000 });
  30  | }
  31  | 
  32  | type Vec3 = [number, number, number];
  33  | async function pose(page: Page, pos: Vec3, target: Vec3): Promise<void> {
  34  |   await page.evaluate(([p, t]) => window.__terrain!.setPose(p, t), [pos, target] as [Vec3, Vec3]);
  35  |   await frames(page, 2);
  36  | }
  37  | 
  38  | /** Decode the page screenshot into RGB pixels once; return a sampler plus a magenta counter. */
  39  | async function grab(page: Page, withData = false) {
> 40  |   const png = (await page.screenshot()).toString('base64');
      |                           ^ Error: page.screenshot: Test timeout of 60000ms exceeded.
  41  |   return page.evaluate(async ([b64, keep]) => {
  42  |     const img = new Image();
  43  |     img.src = `data:image/png;base64,${b64}`;
  44  |     await img.decode();
  45  |     const c = document.createElement('canvas');
  46  |     c.width = img.width;
  47  |     c.height = img.height;
  48  |     const g = c.getContext('2d')!;
  49  |     g.drawImage(img, 0, 0);
  50  |     const d = g.getImageData(0, 0, img.width, img.height).data;
  51  |     let magenta = 0;
  52  |     let nonBackground = 0;
  53  |     for (let i = 0; i < d.length; i += 4) {
  54  |       if (d[i]! > 200 && d[i + 1]! < 110 && d[i + 2]! > 200) magenta++;
  55  |       else nonBackground++;
  56  |     }
  57  |     return { w: img.width, h: img.height, magenta, nonBackground, data: keep ? Array.from(d) : ([] as number[]) };
  58  |   }, [png, withData] as [string, boolean]);
  59  | }
  60  | 
  61  | function rng(seed: number): () => number {
  62  |   let s = seed >>> 0;
  63  |   return () => {
  64  |     s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
  65  |     return s / 4294967296;
  66  |   };
  67  | }
  68  | 
  69  | test('WebGL2: terrain draws as ONE instanced draw call; every debug mode compiles and renders (acceptance 5)', async ({ page }, info) => {
  70  |   const errors = watchErrors(page);
  71  |   await page.goto('/?renderer=webgl2&scene=terrain&debug=1');
  72  |   await ready(page, 5);
  73  |   const caps = await page.evaluate(() => window.__caps as { kind: string; fallbackReason: string | null });
  74  |   expect(caps.kind).toBe('webgl2');
  75  |   const dump = await page.evaluate(() => window.__perf!.dump());
  76  |   expect(dump.draw_calls).toBe(1);
  77  |   const st = await page.evaluate(() => window.__terrain!.stats() as { nodes: number; leaves: number; perLod: number[]; textureBytes: number; firstUpdateMs: number });
  78  |   expect(st.nodes).toBeGreaterThan(5);
  79  |   expect(st.nodes * 8192).toBeGreaterThan(0);
  80  |   expect(st.textureBytes).toBeLessThan(4 * 1024 * 1024);
  81  |   const loadMs = await page.evaluate(() => window.__terrain!.loadMs);
  82  |   console.log(`terrain load (fetch + decode + build tree + first select): ${loadMs.toFixed(0)} ms; first update ${st.firstUpdateMs.toFixed(1)} ms; texture upload ${(st.textureBytes / 1048576).toFixed(2)} MB; draw calls ${dump.draw_calls}; nodes ${st.nodes} (cut ${st.leaves}) per LOD ${st.perLod.join('/')}`);
  83  |   expect(loadMs).toBeLessThanOrEqual(500);
  84  |   for (const mode of ['none', 'lod', 'morph', 'normal', 'height', 'flat']) {
  85  |     await page.evaluate((m) => window.__terrain!.setDebug(m), mode);
  86  |     await frames(page, 3);
  87  |     const g = await grab(page);
  88  |     expect(g.nonBackground, `mode ${mode} produced pixels`).toBeGreaterThan(g.w * g.h * 0.2);
  89  |     await page.screenshot({ path: info.outputPath(`webgl2-${mode}.png`) });
  90  |   }
  91  |   expect(errors).toEqual([]);
  92  | });
  93  | 
  94  | test('GPU-sampled heights match the CPU surface (height debug mode, top-down, near block and far ring)', async ({ page }) => {
  95  |   const errors = watchErrors(page);
  96  |   await page.goto('/?renderer=webgl2&scene=terrain&terrain_debug=height');
  97  |   await ready(page);
  98  |   const rand = rng(21);
  99  |   const hmin = 1;
  100 |   const hmax = 11;
  101 |   // Poses chosen so the probed ground is in the finest LOD the data supports: the vertex grid then lies on the map
  102 |   // texels and the comparison tests sampling/dequantisation/orientation rather than LOD approximation.
  103 |   const cases: { label: string; poses: Vec3[]; half: [number, number]; tol: number; slope: number; outside: boolean }[] = [
  104 |     {
  105 |       label: 'near block (LOD0, 1 m grid)',
  106 |       poses: [[50, 50, 50], [128, 50, 50], [206, 50, 50], [50, 50, 128], [206, 50, 128], [50, 50, 206], [128, 50, 206], [206, 50, 206], [128, 50, 150]],
  107 |       half: [38, 20],
  108 |       tol: 0.1,
  109 |       slope: 0.12,
  110 |       outside: false,
  111 |     },
  112 |     { label: 'far ring (LOD3, 8 m grid = far texels)', poses: [[1000, 700, 500], [-1200, 700, 300], [300, 700, -900]], half: [150, 120], tol: 0.2, slope: 0.1, outside: true },
  113 |   ];
  114 |   for (const c of cases) {
  115 |     let worst = 0;
  116 |     let sum = 0;
  117 |     let used = 0;
  118 |     for (const p of c.poses) {
  119 |       await pose(page, p, [p[0], 0, p[2] + 0.4]);
  120 |       await frames(page, 1);
  121 |       const g = await grab(page, true);
  122 |       const cand = Array.from({ length: 120 }, (): [number, number] => [p[0] + (rand() * 2 - 1) * c.half[0], p[2] + (rand() * 2 - 1) * c.half[1]]);
  123 |       const probes = await page.evaluate(
  124 |         ([pts, maxSlope, outside]) => {
  125 |           const t = window.__terrain!;
  126 |           const out: { sx: number; sy: number; h: number }[] = [];
  127 |           for (const [x, z] of pts as [number, number][]) {
  128 |             if (outside && x > -20 && x < 276 && z > -20 && z < 276) continue;
  129 |             if (!outside && (x < 0 || x > 256 || z < 0 || z > 256)) continue;
  130 |             const gx = Math.abs(t.heightAt(x + 1, z) - t.heightAt(x - 1, z)) / 2;
  131 |             const gz = Math.abs(t.heightAt(x, z + 1) - t.heightAt(x, z - 1)) / 2;
  132 |             if (Math.max(gx, gz) > (maxSlope as number)) continue;
  133 |             const s = t.project(x, z);
  134 |             out.push({ sx: s.x, sy: s.y, h: t.heightAt(x, z) });
  135 |           }
  136 |           return out;
  137 |         },
  138 |         [cand, c.slope, c.outside] as const,
  139 |       );
  140 |       for (const q of probes) {
```