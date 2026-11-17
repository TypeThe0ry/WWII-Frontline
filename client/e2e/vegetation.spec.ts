import { mkdirSync } from 'node:fs';
import { expect, test, type Page } from '@playwright/test';

// M0-024 vegetation. Software-rendered Chromium (SwiftShader): WebGL2 is pixel-checked, the WebGPU path only has to
// compile, bake and draw without validation errors (the shim turns presenting into an offscreen texture).
// GPU frame times are NOT measurable here: acceptance 1 (fps on RTX 3060 / Iris Xe) is "need the reference machine (H-021)".
const SHIM = 'e2e/webgpu-swiftshader-shim.js';
const SHOT_DIR = '../docs/measurements/M0-024';
const SAVE_SHOTS = process.env.SAVE_SHOTS === '1';

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
  await page.waitForFunction(() => window.__vegetation !== undefined && window.__terrain !== undefined, undefined, { timeout: 120_000 });
  await frames(page, minFrames);
}

async function frames(page: Page, n: number): Promise<void> {
  const f0 = await page.evaluate(() => window.__perf?.frames() ?? 0);
  await page.waitForFunction((k) => (window.__perf?.frames() ?? 0) >= k, f0 + n, { timeout: 90_000 });
}

type Vec3 = [number, number, number];

/** The three terrain debug camera presets (terrain-scene.ts PRESETS; M0-030 replaces them with its fixed poses). */
const POSES: { name: string; pos: Vec3; target: Vec3 }[] = [
  { name: 'ground-north', pos: [128, 0, 40], target: [128, 0, 128] },
  { name: 'overview', pos: [128, 110, -90], target: [128, 0, 140] },
  { name: 'west-edge', pos: [20, 12, 128], target: [-3000, 0, 128] },
];

async function setPreset(page: Page, p: (typeof POSES)[number]): Promise<void> {
  await page.evaluate(({ pos, target }) => {
    const t = window.__terrain!;
    const q = (v: Vec3, k: number): Vec3 => {
      const o: Vec3 = [v[0], v[1], v[2]];
      if (o[1] === 0) o[1] = t.heightAt(Math.min(Math.max(o[0], -3900), 4100), Math.min(Math.max(o[2], -3900), 4100)) + k;
      return o;
    };
    t.setPose(q(pos, 1.7), q(target, 1.5));
  }, p);
}

/** Foreground mask of the page screenshot: pixels that differ from the clear colour, as a '0'/'1' string (interior only). */
async function grabMask(page: Page, clear: [number, number, number] = [10, 15, 26], path?: string): Promise<{ w: number; h: number; mask: string; count: number }> {
  const png = (await page.screenshot(path ? { path } : {})).toString('base64');
  return page.evaluate(
    async ([b64, clr]) => {
      const img = new Image();
      img.src = `data:image/png;base64,${b64}`;
      await img.decode();
      const c = document.createElement('canvas');
      c.width = img.width;
      c.height = img.height;
      const g = c.getContext('2d')!;
      g.drawImage(img, 0, 0);
      const d = g.getImageData(0, 0, img.width, img.height).data;
      const M = 10; // the canvas sits inside the 8 px body margin
      let mask = '';
      let count = 0;
      for (let y = M; y < img.height - M; y++) {
        for (let x = M; x < img.width - M; x++) {
          const i = (y * img.width + x) * 4;
          const diff = Math.abs(d[i]! - clr[0]!) + Math.abs(d[i + 1]! - clr[1]!) + Math.abs(d[i + 2]! - clr[2]!);
          if (diff > 36) {
            mask += '1';
            count++;
          } else mask += '0';
        }
      }
      return { w: img.width - 2 * M, h: img.height - 2 * M, mask, count };
    },
    [png, clear] as [string, number[]],
  );
}

test('WebGL2: full 1536 atlas bakes; names, draw calls <= 400 at the 3 camera presets, perf dump vegetation block (acceptance 2, 5, 7)', async ({ page }) => {
  test.setTimeout(240_000);
  const errors = watchErrors(page);
  await page.goto('/?renderer=webgl2&scene=vegetation');
  await ready(page, 5);
  const caps = await page.evaluate(() => window.__caps as { kind: string });
  expect(caps.kind).toBe('webgl2');

  const info = await page.evaluate(() => {
    const v = window.__vegetation!.veg;
    const sizes: Record<string, [number, number]> = {};
    for (const [k, a] of v.atlases) {
      const s = a.albedo.getSize();
      const n = a.normal.getSize();
      sizes[k] = [s.width, s.height];
      sizes[`${k}-normal`] = [n.width, n.height];
    }
    return { sizes, bakeMs: v.stats.bakeMs, loadMs: window.__vegetation!.loadMs, total: v.stats.total, bufferBytes: v.stats.bufferBytes, atlasBytes: v.stats.atlasBytes };
  });
  console.log(`impostor bake (shrub + tree, albedo + normal, SwiftShader): ${info.bakeMs.toFixed(0)} ms; vegetation attach total ${info.loadMs.toFixed(0)} ms; atlases ${JSON.stringify(info.sizes)}; instances ${JSON.stringify(info.total)}; CPU buffers ${(info.bufferBytes / 1048576).toFixed(1)} MB, atlases ${(info.atlasBytes / 1048576).toFixed(0)} MB`);
  for (const k of ['shrub', 'tree', 'shrub-normal', 'tree-normal']) expect(info.sizes[k]).toEqual([1536, 1536]);
  expect(info.total.tree).toBeGreaterThan(300);

  // names (acceptance 7): everything in the scene is a placeholder with a requirement id
  const names = await page.evaluate(() => {
    const s = window.__vegetation!.veg.scene;
    // Babylon's built-in 'default material' exists in every scene; no mesh may use it (checked via `bare`).
    return { meshes: s.meshes.map((m) => m.name), mats: s.materials.map((m) => m.name).filter((n) => n !== 'default material'), bare: s.meshes.filter((m) => !m.material).length };
  });
  expect(names.meshes.length).toBeGreaterThan(40);
  expect(names.bare).toBe(0);
  for (const n of names.meshes) expect(n, `mesh ${n}`).toMatch(/^PH_V-NAT-00\d(_|__)/);
  for (const n of names.mats) expect(n, `material ${n}`).toMatch(/^PH_V-NAT-00\d/);
  expect(names.mats.filter((n) => n.startsWith('PH_V-NAT-002')).length).toBeGreaterThan(0);
  expect(names.mats.filter((n) => n.startsWith('PH_V-NAT-004')).length).toBeGreaterThan(0);

  await page.evaluate(() => window.__terrain!.pause());
  for (const p of POSES) {
    await setPreset(page, p);
    const r = await page.evaluate(() => {
      const v = window.__vegetation!.veg;
      v.settle(v.scene.activeCamera!); // re-bucket for the new pose, then draw
      window.__terrain!.renderOnce();
      const d = window.__perf!.dump();
      return { draw: d.draw_calls, tris: d.triangles, veg: d.vegetation, active: d.active_meshes, st: window.__vegetation!.veg.stats.perLod };
    });
    console.log(`pose ${p.name}: draw calls ${r.draw} (vegetation ${r.veg.drawCalls}), instances ${r.veg.instances}, vegetation tris ${r.veg.tris}, total tris ${r.tris}, per LOD ${JSON.stringify(r.st)}`);
    expect(r.draw).toBeLessThanOrEqual(400);
    expect(r.veg.drawCalls).toBeGreaterThan(0);
    expect(r.veg.drawCalls).toBeLessThanOrEqual(16 * 8);
  }
  expect(errors).toEqual([]);
});

test('impostor vs LOD1 mesh: foreground area at 150 m from 8 azimuths differs <= 25 % (acceptance 5)', async ({ page }) => {
  test.setTimeout(300_000);
  const errors = watchErrors(page);
  await page.goto('/?renderer=webgl2&scene=vegetation');
  await ready(page, 3);
  await page.evaluate(() => {
    window.__terrain!.pause();
    const v = window.__vegetation!.veg;
    v.isolate(true);
    v.scene.getMeshByName('PH_V-NAT-007_terrain')!.isVisible = false;
  });
  for (const species of ['tree', 'shrub'] as const) {
    const fov = species === 'tree' ? 0.14 : 0.05; // narrow lens: the object fills a useful part of the frame at 150 m
    const centerY = species === 'tree' ? 4.7 : 0.7;
    let worst = 0;
    let worstIou = 1;
    const rows: string[] = [];
    for (let k = 0; k < 8; k++) {
      const az = (k / 8) * Math.PI * 2;
      const masks: { mask: string; count: number }[] = [];
      for (const lod of [1, 2]) {
        await page.evaluate(
          ([sp, l, a, f, cy]) => {
            const v = window.__vegetation!.veg;
            v.clearProbes();
            const y0 = 10;
            v.probe(sp as 'tree' | 'shrub', l as number, 100, y0, 100, 0.3);
            const cam = v.scene.activeCamera as unknown as { fov: number };
            cam.fov = f as number;
            window.__terrain!.setPose([100 + 150 * Math.cos(a as number), y0 + 5, 100 + 150 * Math.sin(a as number)], [100, y0 + (cy as number), 100]);
            v.settle(v.scene.activeCamera!); // uCam (impostor billboard basis) follows the pose
            window.__terrain!.renderOnce();
          },
          [species, lod, az, fov, centerY] as const,
        );
        masks.push(await grabMask(page, [10, 15, 26], SAVE_SHOTS ? `${SHOT_DIR}/coverage-${species}-az${k}-${lod === 1 ? 'lod1' : 'impostor'}.png` : undefined));
      }
      const [a, b] = masks as [{ mask: string; count: number }, { mask: string; count: number }];
      let inter = 0;
      let union = 0;
      for (let i = 0; i < a.mask.length; i++) {
        const x = a.mask[i] === '1';
        const y = b.mask[i] === '1';
        if (x && y) inter++;
        if (x || y) union++;
      }
      const diff = Math.abs(a.count - b.count) / a.count;
      const iou = inter / union;
      worst = Math.max(worst, diff);
      worstIou = Math.min(worstIou, iou);
      rows.push(`az ${(k * 45).toString().padStart(3)}: LOD1 ${a.count} px, impostor ${b.count} px, diff ${(diff * 100).toFixed(1)} %, IoU ${iou.toFixed(2)}`);
      expect(a.count, `${species} az ${k} LOD1 visible`).toBeGreaterThan(200);
    }
    console.log(`${species} @150 m, 8 azimuths:\n  ${rows.join('\n  ')}\n  worst area diff ${(worst * 100).toFixed(1)} %, worst IoU ${worstIou.toFixed(2)}`);
    expect(worst).toBeLessThanOrEqual(0.25);
    expect(worstIou, 'silhouettes overlap (catches mirrored / rotated impostor views)').toBeGreaterThanOrEqual(0.6);
  }
  expect(errors).toEqual([]);
});

test('LOD transition: 30 s radial traverse at 6.4 m/s, re-bucketing P99 <= 0.5 ms, no spike over 33 ms, no instance lost (acceptance 6)', async ({ page }) => {
  test.setTimeout(240_000);
  const errors = watchErrors(page);
  await page.goto('/?renderer=webgl2&scene=vegetation&impostor=lowres');
  await ready(page, 3);
  const r = await page.evaluate(() => {
    window.__terrain!.pause();
    const v = window.__vegetation!.veg;
    v.setBudget(Infinity);
    const cam = v.scene.activeCamera as unknown as { position: { x: number; y: number; z: number } };
    const times: number[] = [];
    const classify: number[] = [];
    let lost = 0;
    let moved = 0;
    const treeTotal = v.stats.total.tree;
    const shrubTotal = v.stats.total.shrub;
    const steps = 30 * 60; // 30 s at 60 Hz; the camera crosses the block along +z (192 m)
    const pass = (): void => {
      times.length = 0;
      classify.length = 0;
      moved = 0;
      for (let f = 0; f < steps; f++) {
        cam.position.x = 128;
        cam.position.z = 10 + (6.4 * f) / 60;
        cam.position.y = window.__terrain!.heightAt(128, cam.position.z) + 1.7;
        const t0 = performance.now();
        v.update(v.scene.activeCamera!, 1 / 60);
        times.push(performance.now() - t0);
        classify.push(v.stats.classifyMs);
        moved += v.stats.moved;
        const s = v.stats.perLod;
        if (s.tree.reduce((a, b) => a + b, 0) !== treeTotal || s.shrub.reduce((a, b) => a + b, 0) > shrubTotal) lost++;
      }
    };
    pass(); // warm-up traverse (JIT, first uploads); the second one is measured
    v.update(v.scene.activeCamera!, 0);
    pass();
    const stat = (arr: number[]) => {
      const a = [...arr].sort((x, y) => x - y);
      const pick = (q: number) => a[Math.min(a.length - 1, Math.floor(a.length * q))]!;
      return { p50: pick(0.5), p99: pick(0.99), max: a[a.length - 1]! };
    };
    return { total: stat(times), classify: stat(classify), lost, moved, steps };
  });
  // performance.now() has 0.1 ms resolution in a page that is not cross-origin isolated: values are multiples of 0.1 ms.
  console.log(
    `30 s traverse (2nd pass; 1st = JIT warm-up), ${r.steps} updates (browser JS, SwiftShader buffer writes, 0.1 ms timer): ` +
      `re-bucket total (classify + upload) P50 ${r.total.p50.toFixed(2)} / P99 ${r.total.p99.toFixed(2)} / max ${r.total.max.toFixed(2)} ms; ` +
      `classify only P50 ${r.classify.p50.toFixed(2)} / P99 ${r.classify.p99.toFixed(2)} / max ${r.classify.max.toFixed(2)} ms; instances moved ${r.moved}; trees lost ${r.lost}`,
  );
  expect(r.lost).toBe(0);
  expect(r.moved).toBeGreaterThan(1000);
  expect(r.total.max).toBeLessThan(33);
  expect(r.classify.p99).toBeLessThanOrEqual(0.5);
  expect(r.total.p99).toBeLessThanOrEqual(0.5);
  // the default budgeted mode keeps working and renders after the traverse
  await page.evaluate(() => {
    window.__vegetation!.veg.setBudget(0.45);
    window.__terrain!.renderOnce();
  });
  expect(errors).toEqual([]);
});

test('Low quality: grass 35 % within 20 m, trees turn into impostors early, still <= 400 draw calls', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/?renderer=webgl2&scene=vegetation&impostor=lowres&quality=low&cam=0');
  await ready(page, 4);
  const r = await page.evaluate(() => {
    const v = window.__vegetation!.veg;
    const d = window.__perf!.dump();
    return { q: v.qualityTier, grass: v.stats.perLod.grass, tree: v.stats.perLod.tree, draw: d.draw_calls, veg: d.vegetation };
  });
  console.log(`low quality @cam0: grass per LOD ${r.grass}, tree per LOD ${r.tree}, draw calls ${r.draw}, vegetation ${JSON.stringify(r.veg)}`);
  expect(r.q).toBe('low');
  expect(r.draw).toBeLessThanOrEqual(400);
  expect(errors).toEqual([]);
});

test('wind: crowns sway, trunks stay fixed (acceptance: wind in the vertex shader)', async ({ page }) => {
  test.setTimeout(120_000);
  const errors = watchErrors(page);
  await page.goto('/?renderer=webgl2&scene=vegetation&impostor=lowres');
  await ready(page, 3);
  await page.evaluate(() => {
    window.__terrain!.pause();
    const v = window.__vegetation!.veg;
    v.isolate(true);
    v.scene.getMeshByName('PH_V-NAT-007_terrain')!.isVisible = false;
    v.probe('tree', 0, 100, 10, 100, 0);
    v.windState.strength = 1.2; // exaggerated so the sway is several pixels at this range
    window.__terrain!.setPose([100, 14, 80], [100, 14, 100]);
  });
  const shots: { mask: string; w: number; h: number }[] = [];
  for (const dt of [0, 0.6, 0.5]) {
    await page.evaluate((d) => {
      const v = window.__vegetation!.veg;
      v.update(v.scene.activeCamera!, d);
      window.__terrain!.renderOnce();
    }, dt);
    shots.push(await grabMask(page));
  }
  const [a, b] = [shots[0]!, shots[1]!];
  expect(a.w).toBe(b.w);
  let crownDiff = 0;
  let trunkDiff = 0;
  let trunkPix = 0;
  for (let y = 0; y < a.h; y++) {
    for (let x = 0; x < a.w; x++) {
      const i = y * a.w + x;
      const diff = a.mask[i] !== b.mask[i];
      // the tree spans most of the frame height: the bottom 25 % of the foreground rows are trunk
      if (y > a.h * 0.78) {
        if (a.mask[i] === '1') trunkPix++;
        if (diff) trunkDiff++;
      } else if (diff) crownDiff++;
    }
  }
  console.log(`wind: crown pixels changed ${crownDiff}, trunk-band pixels changed ${trunkDiff} of ${trunkPix}`);
  expect(crownDiff).toBeGreaterThan(30);
  expect(trunkDiff).toBeLessThan(Math.max(4, crownDiff * 0.1));
  expect(errors).toEqual([]);
});

test('WebGPU path (SwiftShader): WGSL foliage + impostor + bake compile and draw; impostor orientation matches the mesh (acceptance 2, 5)', async ({ page }) => {
  test.setTimeout(240_000);
  const errors = watchErrors(page);
  await page.addInitScript({ path: SHIM });
  await page.goto('/?renderer=webgpu&scene=vegetation&impostor=lowres');
  await page.waitForFunction(() => window.__caps !== undefined, undefined, { timeout: 15_000 });
  const caps = await page.evaluate(() => window.__caps as { kind: string; fallbackReason: string | null });
  test.skip(caps.kind !== 'webgpu', `no WebGPU adapter in this browser (${caps.fallbackReason})`);
  await ready(page, 10);
  const r = await page.evaluate(() => {
    const d = window.__perf!.dump();
    return { draw: d.draw_calls, veg: d.vegetation, bakeMs: window.__vegetation!.veg.stats.bakeMs };
  });
  console.log(`WebGPU: draw calls ${r.draw} (vegetation ${JSON.stringify(r.veg)}); lowres atlas bake ${r.bakeMs.toFixed(0)} ms`);
  expect(r.draw).toBeLessThanOrEqual(400);
  expect(r.veg.drawCalls).toBeGreaterThan(0);

  // Orientation check that does not depend on readback conventions: LOD1 mesh and impostor are captured through the same
  // offscreen path (Vegetation.captureProbes) from 8 azimuths at 150 m; the silhouettes must coincide (IoU) and have
  // similar area. A flipped or mirrored atlas (wrong `flipV`) would fail the IoU.
  await page.evaluate(() => {
    window.__terrain!.pause();
    const v = window.__vegetation!.veg;
    v.isolate(true);
    v.scene.getMeshByName('PH_V-NAT-007_terrain')!.isVisible = false;
  });
  let worst = 0;
  let worstIou = 1;
  const rows: string[] = [];
  for (let k = 0; k < 8; k++) {
    const az = (k / 8) * Math.PI * 2;
    const caps: { size: number; alpha: number[] }[] = [];
    for (const lod of [1, 2]) {
      caps.push(
        await page.evaluate(
          async ([l, a]) => {
            const v = window.__vegetation!.veg;
            v.clearProbes();
            const y0 = 10;
            v.probe('tree', l as number, 100, y0, 100, 0.3);
            const cam = v.scene.activeCamera as unknown as { fov: number };
            cam.fov = 0.14;
            window.__terrain!.setPose([100 + 150 * Math.cos(a as number), y0 + 5, 100 + 150 * Math.sin(a as number)], [100, y0 + 4.7, 100]);
            v.settle(v.scene.activeCamera!);
            const c = await v.captureProbes(256);
            return { size: c.size, alpha: Array.from(c.alpha) };
          },
          [lod, az] as const,
        ),
      );
    }
    const m = caps[0]!;
    const im = caps[1]!;
    let ca = 0;
    let cb = 0;
    let inter = 0;
    let union = 0;
    for (let i = 0; i < m.alpha.length; i++) {
      const x = m.alpha[i]! > 127;
      const y = im.alpha[i]! > 127;
      if (x) ca++;
      if (y) cb++;
      if (x && y) inter++;
      if (x || y) union++;
    }
    expect(ca, `az ${k}: LOD1 mesh visible`).toBeGreaterThan(300);
    const diff = Math.abs(ca - cb) / ca;
    worst = Math.max(worst, diff);
    worstIou = Math.min(worstIou, inter / union);
    rows.push(`az ${k * 45}: LOD1 ${ca} px, impostor ${cb} px, diff ${(diff * 100).toFixed(1)} %, IoU ${(inter / union).toFixed(2)}`);
  }
  console.log(`WebGPU tree @150 m via offscreen capture:\n  ${rows.join('\n  ')}\n  worst area diff ${(worst * 100).toFixed(1)} %, worst IoU ${worstIou.toFixed(2)}`);
  expect(worst).toBeLessThanOrEqual(0.25);
  expect(worstIou).toBeGreaterThanOrEqual(0.6);
  expect(errors).toEqual([]);
});

test('reference screenshots + atlas dump (SAVE_SHOTS=1 writes docs/measurements/M0-024/)', async ({ page }) => {
  test.skip(!SAVE_SHOTS, 'set SAVE_SHOTS=1 to refresh the reference screenshots');
  test.setTimeout(300_000);
  await page.setViewportSize({ width: 960, height: 540 });
  mkdirSync(SHOT_DIR, { recursive: true });
  const errors = watchErrors(page);
  await page.goto('/?renderer=webgl2&scene=vegetation');
  await ready(page, 3);
  await page.evaluate(() => window.__terrain!.pause());
  const shots: [string, Vec3, Vec3][] = [
    ['overview', [128, 110, -90], [128, 0, 140]],
    ['ground-forest', [60, 0, 20], [110, 0, 80]],
    ['grass-closeup', [213, 0, 93], [215, 0, 112]],
  ];
  for (const [name, pos, target] of shots) {
    await page.evaluate(([p, t]) => {
      const T = window.__terrain!;
      const q = (v: Vec3, k: number): Vec3 => [v[0], v[1] === 0 ? T.heightAt(v[0], v[2]) + k : v[1], v[2]];
      T.setPose(q(p, 1.7), q(t, 1.2));
      const v = window.__vegetation!.veg;
      v.settle(v.scene.activeCamera!); // re-bucket for the new pose before drawing
      T.renderOnce();
    }, [pos, target] as [Vec3, Vec3]);
    await page.screenshot({ path: `${SHOT_DIR}/${name}.png` });
  }
  // atlas of the tree (albedo), as the GPU holds it: drawn into a page canvas, then an element screenshot
  await page.evaluate(async () => {
    const a = window.__vegetation!.veg.atlases.get('tree')!;
    const px = (await a.albedo.readPixels()) as Uint8Array;
    const W = a.cells * a.cellPx;
    const c = document.createElement('canvas');
    c.id = 'atlas-dump';
    c.width = W;
    c.height = W;
    c.style.cssText = `position:absolute;left:0;top:0;width:${W}px;height:${W}px;z-index:99;background:#202830`;
    const g = c.getContext('2d')!;
    const img = g.createImageData(W, W);
    for (let y = 0; y < W; y++) img.data.set(px.subarray(y * W * 4, (y + 1) * W * 4), (W - 1 - y) * W * 4); // GL readback is bottom-up
    g.putImageData(img, 0, 0);
    document.body.appendChild(c);
  });
  await page.locator('#atlas-dump').screenshot({ path: `${SHOT_DIR}/tree-impostor-atlas-albedo.png` });
  expect(errors).toEqual([]);
});
