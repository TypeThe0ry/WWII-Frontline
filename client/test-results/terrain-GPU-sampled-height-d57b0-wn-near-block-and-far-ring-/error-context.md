# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: terrain.spec.ts >> GPU-sampled heights match the CPU surface (height debug mode, top-down, near block and far ring)
- Location: e2e/terrain.spec.ts:94:1

# Error details

```
Error: expect(received).toBeLessThanOrEqual(expected)

Expected: <= 0.1
Received:    9.221654448369181
```

# Test source

```ts
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
  141 |         const px = Math.round(q.sx);
  142 |         const py = Math.round(q.sy);
  143 |         if (px < 2 || py < 2 || px >= g.w - 2 || py >= g.h - 2) continue;
  144 |         const rendered = hmin + (g.data[(py * g.w + px) * 4]! / 255) * (hmax - hmin);
  145 |         const err = Math.abs(rendered - q.h);
  146 |         worst = Math.max(worst, err);
  147 |         sum += err;
  148 |         used++;
  149 |       }
  150 |     }
  151 |     console.log(`${c.label}: ${used} probes on gentle ground, GPU-vs-CPU height error max ${worst.toFixed(3)} m, mean ${(sum / used).toFixed(3)} m (8-bit gray = ${((hmax - hmin) / 255).toFixed(3)} m/step)`);
  152 |     expect(used).toBeGreaterThan(40);
> 153 |     expect(worst).toBeLessThanOrEqual(c.tol);
      |                   ^ Error: expect(received).toBeLessThanOrEqual(expected)
  154 |   }
  155 |   expect(errors).toEqual([]);
  156 | });
  157 | 
  158 | test('no cracks: flat shading over a magenta clear colour, random poses; control without edge masks does crack', async ({ page }) => {
  159 |   test.setTimeout(240_000);
  160 |   const errors = watchErrors(page);
  161 |   await page.goto('/?renderer=webgl2&scene=terrain&terrain_debug=flat');
  162 |   await ready(page);
  163 |   const rand = rng(77);
  164 |   const poses: [Vec3, Vec3][] = [];
  165 |   for (let i = 0; i < 14; i++) {
  166 |     const x = rand() * 256;
  167 |     const z = rand() * 256;
  168 |     const yaw = rand() * Math.PI * 2;
  169 |     const y = 2 + rand() * 25;
  170 |     poses.push([[x, y, z], [x + Math.sin(yaw) * 200, 6, z + Math.cos(yaw) * 200]]);
  171 |   }
  172 |   // Low grazing views from outside the block toward the far field and uphill (the geometry where a crack is see-through).
  173 |   for (let i = 0; i < 6; i++) {
  174 |     const yaw = rand() * Math.PI * 2;
  175 |     poses.push([[-300 + rand() * 900, 1.7 + rand() * 6, -300 + rand() * 900], [128 + Math.sin(yaw) * 900, 4, 128 + Math.cos(yaw) * 900]]);
  176 |   }
  177 |   let stitched = 0;
  178 |   let control = 0;
  179 |   for (const [p, t] of poses) {
  180 |     await page.evaluate(() => window.__terrain!.setStitch(true));
  181 |     await pose(page, p, t);
  182 |     stitched += (await grab(page)).magenta;
  183 |     await page.evaluate(() => window.__terrain!.setStitch(false));
  184 |     await pose(page, p, t);
  185 |     control += (await grab(page)).magenta;
  186 |   }
  187 |   console.log(`magenta (hole) pixels over ${poses.length} poses: stitched ${stitched}, control (no edge masks) ${control}`);
  188 |   expect(stitched).toBe(0);
  189 |   expect(control, 'detector sensitivity: the un-stitched control must show holes').toBeGreaterThan(0);
  190 |   expect(errors).toEqual([]);
  191 | });
  192 | 
  193 | test('far-ring edge is visible 4 km out and is a clean silhouette (acceptance 7, screenshot)', async ({ page }, info) => {
  194 |   const errors = watchErrors(page);
  195 |   await page.goto('/?renderer=webgl2&scene=terrain&terrain_debug=flat');
  196 |   await ready(page);
  197 |   await pose(page, [128, 12, 128], [-3000, 12, 128]);
  198 |   const g = await grab(page);
  199 |   // Scan the middle column from the bottom: terrain (white) must end at the horizon with background above it.
  200 |   const x = g.w >> 1;
  201 |   let edge = -1;
  202 |   for (let y = g.h - 1; y > 0; y--) {
  203 |     const i = (y * g.w + x) * 4;
  204 |     if (g.data[i]! > 200 && g.data[i + 1]! < 110) {
  205 |       edge = y;
  206 |       break;
  207 |     }
  208 |   }
  209 |   expect(edge, 'background (magenta) found above the terrain silhouette').toBeGreaterThan(0);
  210 |   expect(edge).toBeLessThan(g.h * 0.75);
  211 |   console.log(`far ring silhouette row ${edge}/${g.h}`);
  212 |   await page.screenshot({ path: info.outputPath('far-edge-flat.png') });
  213 |   expect(errors).toEqual([]);
  214 | });
  215 | 
  216 | test('WebGPU path (SwiftShader): WGSL terrain compiles and draws without validation errors (pixels not checked)', async ({ page }) => {
  217 |   const errors = watchErrors(page);
  218 |   await page.addInitScript({ path: SHIM });
  219 |   await page.goto('/?renderer=webgpu&scene=terrain&debug=1');
  220 |   await page.waitForFunction(() => window.__caps !== undefined, undefined, { timeout: 15_000 });
  221 |   const caps = await page.evaluate(() => window.__caps as { kind: string; fallbackReason: string | null });
  222 |   test.skip(caps.kind !== 'webgpu', `no WebGPU adapter in this browser (${caps.fallbackReason})`);
  223 |   await ready(page, 20);
  224 |   const st = await page.evaluate(() => window.__terrain!.stats() as { nodes: number });
  225 |   expect(st.nodes).toBeGreaterThan(5);
  226 |   for (const mode of ['lod', 'morph', 'normal', 'none']) {
  227 |     await page.evaluate((m) => window.__terrain!.setDebug(m), mode);
  228 |     await frames(page, 5);
  229 |   }
  230 |   const d = await page.evaluate(() => window.__perf!.dump());
  231 |   expect(d.draw_calls).toBe(1);
  232 |   expect(errors).toEqual([]);
  233 | });
  234 | 
  235 | test('reference screenshots: orbit around the block in lod/shaded modes (SAVE_SHOTS=1 writes docs/measurements/M0-023/)', async ({ page }) => {
  236 |   test.skip(!SAVE_SHOTS, 'set SAVE_SHOTS=1 to refresh the reference screenshots');
  237 |   test.setTimeout(240_000);
  238 |   await page.setViewportSize({ width: 960, height: 540 });
  239 |   mkdirSync(SHOT_DIR, { recursive: true });
  240 |   const errors = watchErrors(page);
  241 |   await page.goto('/?renderer=webgl2&scene=terrain&terrain_debug=lod');
  242 |   await ready(page);
  243 |   for (let i = 0; i < 8; i++) {
  244 |     const a = (i / 8) * Math.PI * 2;
  245 |     await pose(page, [128 + Math.sin(a) * 150, 45, 128 + Math.cos(a) * 150], [128, 4, 128]);
  246 |     await page.screenshot({ path: `${SHOT_DIR}/orbit-lod-${i}.png` });
  247 |   }
  248 |   await page.evaluate(() => window.__terrain!.setDebug('none'));
  249 |   await pose(page, [128, 12, 40], [128, 5, 200]);
  250 |   await page.screenshot({ path: `${SHOT_DIR}/ground-shaded.png` });
  251 |   await pose(page, [128, 14, 128], [-3000, 14, 128]);
  252 |   await page.screenshot({ path: `${SHOT_DIR}/far-ring-edge.png` });
  253 |   await page.evaluate(() => window.__terrain!.setDebug('normal'));
```