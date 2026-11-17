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

Expected: <= 0.12
Received:    3.934728254850401
```

# Test source

```ts
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
  57  |     return { w: img.width, h: img.height, magenta, nonBackground, data: Array.from(d) };
  58  |   }, png);
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
  101 |   const cases: { label: string; pos: Vec3; target: Vec3; pts: () => [number, number]; tol: number; slope: number }[] = [
  102 |     { label: 'near block', pos: [128, 400, 128], target: [128, 0, 128.5], pts: () => [rand() * 256, rand() * 256], tol: 0.12, slope: 0.04 },
  103 |     { label: 'far ring', pos: [128, 5000, 128], target: [128, 0, 128.5], pts: () => [-1500 + rand() * 3000, -700 + rand() * 1700], tol: 0.45, slope: 0.02 },
  104 |   ];
  105 |   for (const c of cases) {
  106 |     await pose(page, c.pos, c.target);
  107 |     await frames(page, 1);
  108 |     const g = await grab(page);
  109 |     // Candidate points on gentle ground only (1 px of projection error on a steep slope would dominate the comparison).
  110 |     const cand = Array.from({ length: 600 }, () => c.pts());
  111 |     const probes = await page.evaluate(
  112 |       ([pts, maxSlope, label]) => {
  113 |         const t = window.__terrain!;
  114 |         const out: { x: number; z: number; sx: number; sy: number; h: number }[] = [];
  115 |         for (const [x, z] of pts as [number, number][]) {
  116 |           if (label === 'far ring' && x > -20 && x < 276 && z > -20 && z < 276) continue; // this case probes outside the block
  117 |           const h = t.heightAt(x, z);
  118 |           const gx = Math.abs(t.heightAt(x + 4, z) - t.heightAt(x - 4, z)) / 8;
  119 |           const gz = Math.abs(t.heightAt(x, z + 4) - t.heightAt(x, z - 4)) / 8;
  120 |           if (Math.max(gx, gz) > (maxSlope as number)) continue;
  121 |           const s = t.project(x, z);
  122 |           out.push({ x, z, sx: s.x, sy: s.y, h });
  123 |         }
  124 |         return out;
  125 |       },
  126 |       [cand, c.slope, c.label] as const,
  127 |     );
  128 |     let worst = 0;
  129 |     let sum = 0;
  130 |     let used = 0;
  131 |     for (const q of probes) {
  132 |       const px = Math.round(q.sx);
  133 |       const py = Math.round(q.sy);
  134 |       if (px < 2 || py < 2 || px >= g.w - 2 || py >= g.h - 2) continue;
  135 |       const rendered = hmin + (g.data[(py * g.w + px) * 4]! / 255) * (hmax - hmin);
  136 |       const err = Math.abs(rendered - q.h);
  137 |       worst = Math.max(worst, err);
  138 |       sum += err;
  139 |       used++;
  140 |     }
  141 |     console.log(`${c.label}: ${used} probes on gentle ground, GPU-vs-CPU height error max ${worst.toFixed(3)} m, mean ${(sum / used).toFixed(3)} m (8-bit gray = ${((hmax - hmin) / 255).toFixed(3)} m/step)`);
  142 |     expect(used).toBeGreaterThan(40);
> 143 |     expect(worst).toBeLessThanOrEqual(c.tol);
      |                   ^ Error: expect(received).toBeLessThanOrEqual(expected)
  144 |   }
  145 |   expect(errors).toEqual([]);
  146 | });
  147 | 
  148 | test('no cracks: flat shading over a magenta clear colour, random poses; control without edge masks does crack', async ({ page }) => {
  149 |   test.setTimeout(240_000);
  150 |   const errors = watchErrors(page);
  151 |   await page.goto('/?renderer=webgl2&scene=terrain&terrain_debug=flat');
  152 |   await ready(page);
  153 |   const rand = rng(77);
  154 |   const poses: [Vec3, Vec3][] = [];
  155 |   for (let i = 0; i < 14; i++) {
  156 |     const x = rand() * 256;
  157 |     const z = rand() * 256;
  158 |     const yaw = rand() * Math.PI * 2;
  159 |     const y = 2 + rand() * 25;
  160 |     poses.push([[x, y, z], [x + Math.sin(yaw) * 200, 6, z + Math.cos(yaw) * 200]]);
  161 |   }
  162 |   // Low grazing views from outside the block toward the far field and uphill (the geometry where a crack is see-through).
  163 |   for (let i = 0; i < 6; i++) {
  164 |     const yaw = rand() * Math.PI * 2;
  165 |     poses.push([[-300 + rand() * 900, 1.7 + rand() * 6, -300 + rand() * 900], [128 + Math.sin(yaw) * 900, 4, 128 + Math.cos(yaw) * 900]]);
  166 |   }
  167 |   let stitched = 0;
  168 |   let control = 0;
  169 |   for (const [p, t] of poses) {
  170 |     await page.evaluate(() => window.__terrain!.setStitch(true));
  171 |     await pose(page, p, t);
  172 |     stitched += (await grab(page)).magenta;
  173 |     await page.evaluate(() => window.__terrain!.setStitch(false));
  174 |     await pose(page, p, t);
  175 |     control += (await grab(page)).magenta;
  176 |   }
  177 |   console.log(`magenta (hole) pixels over ${poses.length} poses: stitched ${stitched}, control (no edge masks) ${control}`);
  178 |   expect(stitched).toBe(0);
  179 |   expect(control, 'detector sensitivity: the un-stitched control must show holes').toBeGreaterThan(0);
  180 |   expect(errors).toEqual([]);
  181 | });
  182 | 
  183 | test('far-ring edge is visible 4 km out and is a clean silhouette (acceptance 7, screenshot)', async ({ page }, info) => {
  184 |   const errors = watchErrors(page);
  185 |   await page.goto('/?renderer=webgl2&scene=terrain&terrain_debug=flat');
  186 |   await ready(page);
  187 |   await pose(page, [128, 12, 128], [-3000, 12, 128]);
  188 |   const g = await grab(page);
  189 |   // Scan the middle column from the bottom: terrain (white) must end at the horizon with background above it.
  190 |   const x = g.w >> 1;
  191 |   let edge = -1;
  192 |   for (let y = g.h - 1; y > 0; y--) {
  193 |     const i = (y * g.w + x) * 4;
  194 |     if (g.data[i]! > 200 && g.data[i + 1]! < 110) {
  195 |       edge = y;
  196 |       break;
  197 |     }
  198 |   }
  199 |   expect(edge, 'background (magenta) found above the terrain silhouette').toBeGreaterThan(0);
  200 |   expect(edge).toBeLessThan(g.h * 0.75);
  201 |   console.log(`far ring silhouette row ${edge}/${g.h}`);
  202 |   await page.screenshot({ path: info.outputPath('far-edge-flat.png') });
  203 |   expect(errors).toEqual([]);
  204 | });
  205 | 
  206 | test('WebGPU path (SwiftShader): WGSL terrain compiles and draws without validation errors (pixels not checked)', async ({ page }) => {
  207 |   const errors = watchErrors(page);
  208 |   await page.addInitScript({ path: SHIM });
  209 |   await page.goto('/?renderer=webgpu&scene=terrain&debug=1');
  210 |   await page.waitForFunction(() => window.__caps !== undefined, undefined, { timeout: 15_000 });
  211 |   const caps = await page.evaluate(() => window.__caps as { kind: string; fallbackReason: string | null });
  212 |   test.skip(caps.kind !== 'webgpu', `no WebGPU adapter in this browser (${caps.fallbackReason})`);
  213 |   await ready(page, 20);
  214 |   const st = await page.evaluate(() => window.__terrain!.stats() as { nodes: number });
  215 |   expect(st.nodes).toBeGreaterThan(5);
  216 |   for (const mode of ['lod', 'morph', 'normal', 'none']) {
  217 |     await page.evaluate((m) => window.__terrain!.setDebug(m), mode);
  218 |     await frames(page, 5);
  219 |   }
  220 |   const d = await page.evaluate(() => window.__perf!.dump());
  221 |   expect(d.draw_calls).toBe(1);
  222 |   expect(errors).toEqual([]);
  223 | });
  224 | 
  225 | test('reference screenshots: orbit around the block in lod/shaded modes (SAVE_SHOTS=1 writes docs/measurements/M0-023/)', async ({ page }) => {
  226 |   test.skip(!SAVE_SHOTS, 'set SAVE_SHOTS=1 to refresh the reference screenshots');
  227 |   test.setTimeout(240_000);
  228 |   await page.setViewportSize({ width: 960, height: 540 });
  229 |   mkdirSync(SHOT_DIR, { recursive: true });
  230 |   const errors = watchErrors(page);
  231 |   await page.goto('/?renderer=webgl2&scene=terrain&terrain_debug=lod');
  232 |   await ready(page);
  233 |   for (let i = 0; i < 8; i++) {
  234 |     const a = (i / 8) * Math.PI * 2;
  235 |     await pose(page, [128 + Math.sin(a) * 150, 45, 128 + Math.cos(a) * 150], [128, 4, 128]);
  236 |     await page.screenshot({ path: `${SHOT_DIR}/orbit-lod-${i}.png` });
  237 |   }
  238 |   await page.evaluate(() => window.__terrain!.setDebug('none'));
  239 |   await pose(page, [128, 12, 40], [128, 5, 200]);
  240 |   await page.screenshot({ path: `${SHOT_DIR}/ground-shaded.png` });
  241 |   await pose(page, [128, 14, 128], [-3000, 14, 128]);
  242 |   await page.screenshot({ path: `${SHOT_DIR}/far-ring-edge.png` });
  243 |   await page.evaluate(() => window.__terrain!.setDebug('normal'));
```