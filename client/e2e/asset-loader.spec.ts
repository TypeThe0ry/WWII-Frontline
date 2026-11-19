import { expect, test } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

interface PackServerInfo {
  port: number;
  demo: { root: string; manifestPath: string; probePath: string; assets: string[]; textures: string[] };
  bench100mb: { root: string; totalPayloadBytes: number; packCount: number; meshIds: string[]; texIds: string[]; meshEntryBytes: number; texEntryBytes: number };
  upload200: { root: string; count: number; entryBytes: number; totalBytes: number; meshIds: string[] };
}

let serverProc: ChildProcess | undefined;
let serverInfo: PackServerInfo;
let workDir: string;

const repoRoot = resolve(process.env.FRONTLINE_REPO_ROOT ?? '..');

function startPackServer(dir: string): Promise<{ proc: ChildProcess; info: PackServerInfo }> {
  return new Promise((resolveReady, rejectReady) => {
    const script = join(repoRoot, 'tools/asset-pipeline/scripts/e2e-pack-server.mjs');
    const proc = spawn('node', [script, dir], { stdio: ['ignore', 'pipe', 'pipe'], cwd: repoRoot });
    let buf = '';
    let errBuf = '';
    let settled = false;
    const timer = setTimeout(() => {
      if (!settled) {
        settled = true;
        proc.kill('SIGTERM');
        rejectReady(new Error(`e2e-pack-server timed out:\n${buf}\n${errBuf}`));
      }
    }, 60_000);
    proc.stdout?.on('data', (chunk) => {
      buf += chunk.toString();
      const lines = buf.split('\n');
      for (const line of lines) {
        if (line.startsWith('E2E_PACK_SERVER_READY ') && !settled) {
          settled = true;
          clearTimeout(timer);
          resolveReady({ proc, info: JSON.parse(line.slice('E2E_PACK_SERVER_READY '.length)) as PackServerInfo });
        }
      }
    });
    proc.stderr?.on('data', (chunk) => {
      errBuf += chunk.toString();
    });
    proc.once('exit', () => {
      if (!settled) {
        settled = true;
        clearTimeout(timer);
        rejectReady(new Error(`e2e-pack-server exited early:\n${buf}\n${errBuf}`));
      }
    });
  });
}

test.describe.configure({ mode: 'serial', timeout: 120_000 });

test.beforeAll(async () => {
  workDir = mkdtempSync(join(tmpdir(), 'fl-asset-e2e-'));
  const started = await startPackServer(workDir);
  serverProc = started.proc;
  serverInfo = started.info;
});

test.afterAll(async () => {
  serverProc?.kill('SIGTERM');
  if (workDir) rmSync(workDir, { recursive: true, force: true });
});

async function openProbePage(page: import('@playwright/test').Page): Promise<void> {
  // Real Vite-served empty page on localhost so Chromium 153 Local Network Access allows fetching the local pack server.
  await page.goto('/e2e/asset-loader-probe.html');
}

const MODS = {
  assetsPath: '/src/render/assets/index.ts',
  workerDecoderPath: '/src/render/assets/worker-decoder.ts',
  codecProbePath: '/src/audio/codec-probe.ts',
};

test('M1-001..006: full pipeline -> real Worker decode, BasisU transcode, audio probe/decode, HTTP 206 & 200 fallback', async ({ page }) => {
  await openProbePage(page);
  const origin = `http://localhost:${serverInfo.port}`;
  const res = await page.evaluate(
    async ({ origin, textures, assetsPath, workerDecoderPath, codecProbePath }) => {
      const assetsMod: typeof import('../src/render/assets/index') = await import(assetsPath);
      const { workerDecoder }: typeof import('../src/render/assets/worker-decoder') = await import(workerDecoderPath);
      const { probeCodec, resetCodecProbe }: typeof import('../src/audio/codec-probe') = await import(codecProbePath);
      const { AssetLoader, CacheLayer, detectBackends, isPlaceholder, parseProbeJson } = assetsMod;

      const backends = await detectBackends('0.1.0-e2e');
      const cache = new CacheLayer({ backends });
      const decoder = workerDecoder(2, undefined, { caps: { bc7: true }, basisBase: '/basis' });
      const loader = new AssetLoader({ base: `${origin}/range/demo`, cache, decoder, log: () => undefined });
      await loader.init({ manifestUrl: `${origin}/range/demo/manifest.json`, tier: 'medium' });

      let pumping = true;
      const pump = () => {
        loader.uploads.runFrame();
        if (pumping) requestAnimationFrame(pump);
      };
      requestAnimationFrame(pump);

      try {
        const lod0 = await loader.loadMesh('prop_demo_crate', 0);
        const lod1 = await loader.loadMesh('prop_demo_crate', 1);
        const fpBytes = await loader.loadBytes('wpn_demo_rifle@lod0_fp');
        const tpBytes = await loader.loadBytes('wpn_demo_rifle@lod0_tp');
        const texResults = [];
        for (const tid of textures) {
          const t = await loader.loadTexture(tid);
          texResults.push({
            id: tid,
            placeholder: isPlaceholder(t),
            data: isPlaceholder(t) ? null : (t.data as { format: string; width: number; height: number; levels: { data: Uint8Array }[] }),
          });
        }

        const probeJson = await (await fetch(`${origin}/range/demo/probe.json`)).json();
        const probe = parseProbeJson(probeJson);
        resetCodecProbe();
        const offline = new OfflineAudioContext(1, 48000, 48000);
        const chosenCodec = await probeCodec((b) => offline.decodeAudioData(b), { opus: probe.opus, aac: probe.aac }, true);
        const audioEntry = await loader.loadAudioEntry(`snd_demo_shot@${chosenCodec}`);
        let decodedAudio = null;
        if (!('silent' in audioEntry)) {
          const ab = audioEntry.payload.slice().buffer;
          const decoded = await offline.decodeAudioData(ab);
          const ch0 = decoded.getChannelData(0);
          let sumSq = 0;
          for (let i = 0; i < ch0.length; i++) sumSq += (ch0[i] ?? 0) ** 2;
          decodedAudio = {
            container: audioEntry.container,
            preload: audioEntry.preload,
            durationS: decoded.duration,
            sampleRate: decoded.sampleRate,
            rms: Math.sqrt(sumSq / ch0.length),
          };
        }

        // HTTP 200 fallback (server ignores Range header).
        const loader200 = new AssetLoader({ base: `${origin}/norange/demo`, cache: new CacheLayer(), decoder, log: () => undefined });
        await loader200.init({ manifestUrl: `${origin}/norange/demo/manifest.json`, tier: 'medium' });
        const pump200 = () => {
          loader200.uploads.runFrame();
          if (pumping) requestAnimationFrame(pump200);
        };
        requestAnimationFrame(pump200);
        const lod0From200 = await loader200.loadMesh('prop_demo_crate', 0);

        return {
          cacheMode: loader.stats().cacheMode,
          opfsSync: backends[0]?.sync ?? false,
          lod0Tris: isPlaceholder(lod0) ? 0 : ((lod0.data as { primitives: { indices: Uint32Array }[] }).primitives[0]?.indices.length ?? 0) / 3,
          lod1Tris: isPlaceholder(lod1) ? 0 : ((lod1.data as { primitives: { indices: Uint32Array }[] }).primitives[0]?.indices.length ?? 0) / 3,
          lod0From200Tris: isPlaceholder(lod0From200)
            ? 0
            : ((lod0From200.data as { primitives: { indices: Uint32Array }[] }).primitives[0]?.indices.length ?? 0) / 3,
          fpBytes: fpBytes.byteLength,
          tpBytes: tpBytes.byteLength,
          textures: texResults.map((x) => ({
            id: x.id,
            placeholder: x.placeholder,
            format: x.data?.format,
            width: x.data?.width,
            height: x.data?.height,
            levels: x.data?.levels.length,
          })),
          chosenCodec,
          decodedAudio,
        };
      } finally {
        pumping = false;
        decoder.terminate();
        cache.close();
      }
    },
    { origin, textures: serverInfo.demo.textures, ...MODS },
  );

  expect(res.cacheMode).toBe('opfs');
  expect(res.opfsSync).toBe(true);
  expect(res.lod0Tris).toBeGreaterThan(0);
  expect(res.lod1Tris).toBeGreaterThan(0);
  expect(res.lod0Tris).toBeGreaterThan(res.lod1Tris);
  expect(res.lod0From200Tris).toBe(res.lod0Tris);
  expect(res.fpBytes).toBeGreaterThan(0);
  expect(res.tpBytes).toBeGreaterThan(0);
  expect(res.textures).toHaveLength(3);
  for (const t of res.textures) {
    expect(t.placeholder).toBe(false);
    expect(t.format).toBe('bc7');
    expect(t.width).toBe(64);
    expect(t.height).toBe(64);
    expect(t.levels).toBe(7);
  }
  expect(res.chosenCodec).toBe('opus');
  expect(res.decodedAudio).not.toBeNull();
  expect(res.decodedAudio!.container).toBe('webm-opus');
  expect(res.decodedAudio!.preload).toBe(true);
  expect(Math.abs(res.decodedAudio!.durationS - 1.0)).toBeLessThan(0.05);
  expect(res.decodedAudio!.rms).toBeGreaterThan(0.05);
});

test('M1-006 acceptance 1 & 4 & benchmark: >= 100 MB .fpak cold vs warm OPFS sync load, 0 long tasks, 0 warm network requests', async ({ page, context }) => {
  expect(serverInfo.bench100mb.totalPayloadBytes).toBeGreaterThanOrEqual(100 * 1024 * 1024);
  const origin = `http://localhost:${serverInfo.port}`;

  const runPassInPage = async (label: string, clearBefore: boolean) => {
    await openProbePage(page);
    return page.evaluate(
      async ({ origin, meshIds, texIds, assetsPath, workerDecoderPath, label, clearBefore }) => {
        const { AssetLoader, CacheLayer, defaultPoolSize, isPlaceholder, opfsSyncBackend }: typeof import('../src/render/assets/index') = await import(assetsPath);
        const { workerDecoder }: typeof import('../src/render/assets/worker-decoder') = await import(workerDecoderPath);

        const dirName = 'frontline-assets-bench100mb';
        if (clearBefore) {
          const setupOpfs = await opfsSyncBackend({ dirName });
          await setupOpfs.clear();
          setupOpfs.close?.();
        }

        const opfs = await opfsSyncBackend({ dirName });
        // Keep MemoryLru tiny (4 MB) so warm pass genuinely exercises OPFS FileSystemSyncAccessHandle for all 100+ MB.
        const cache = new CacheLayer({ backends: [opfs], memoryBytes: 4 * 1024 * 1024 });
        const poolSize = defaultPoolSize();
        const rawDecoder = workerDecoder(poolSize, undefined, { caps: { bc7: true }, basisBase: '/basis' });
        let decodeCpuSumMs = 0;
        const decoder = {
          async decode(entryId: string, kind: 'mesh' | 'texture', bytes: Uint8Array) {
            const ds = performance.now();
            const out = await rawDecoder.decode(entryId, kind, bytes);
            decodeCpuSumMs += performance.now() - ds;
            return out;
          },
          terminate: () => rawDecoder.terminate(),
        };
        // Load manifest before resetting stats so network count isolates `.fpak` entry requests.
        const manifest = await (await fetch(`${origin}/range/bench100mb/manifest.json`)).json();
        await fetch(`${origin}/stats/reset`, { method: 'POST' });

        let downloadEndMs = 0;
        let decodeEndMs = 0;
        const loader = new AssetLoader({
          base: `${origin}/range/bench100mb`,
          manifest,
          cache,
          decoder,
          poolSize,
          log: () => undefined,
          onProgress: (ev) => {
            const now = performance.now();
            if (ev.phase === 'download' && ev.loaded > 0) downloadEndMs = now;
            if (ev.phase === 'decode' && ev.loaded > 0) decodeEndMs = now;
          },
        });
        await loader.init({ tier: 'medium' });

        const t0 = performance.now();
        const longTasks: { atMs: number; durationMs: number }[] = [];
        const obs = new PerformanceObserver((list) => {
          for (const e of list.getEntries()) {
            if (e.duration > 50) longTasks.push({ atMs: Number((e.startTime - t0).toFixed(1)), durationMs: Number(e.duration.toFixed(1)) });
          }
        });
        obs.observe({ type: 'longtask', buffered: false });
        let pumping = true;
        const pump = () => {
          loader.uploads.runFrame();
          if (pumping) requestAnimationFrame(pump);
        };
        requestAnimationFrame(pump);

        let tDone: number;
        let loadedBytes = 0;
        let placeholders = 0;
        try {
          const all: Promise<unknown>[] = [];
          for (const id of meshIds) {
            all.push(
              loader.loadMesh(id, 0).then((r) => {
                if (isPlaceholder(r)) placeholders++;
                else loadedBytes += r.bytes.byteLength;
              }),
            );
          }
          for (const tid of texIds) {
            all.push(
              loader.loadTexture(tid).then((r) => {
                if (isPlaceholder(r)) placeholders++;
                else loadedBytes += r.bytes.byteLength;
              }),
            );
          }
          await Promise.all(all);
          loader.flushProgress();
          tDone = performance.now();
        } finally {
          pumping = false;
          // Give PerformanceObserver one macrotask to flush any pending longtask entries.
          await new Promise((r) => setTimeout(r, 30));
          obs.disconnect();
          decoder.terminate();
        }
        const totalMs = tDone - t0;
        const opfsUsageBytes = (await opfs.usage?.()) ?? 0;
        const serverStats = (await (await fetch(`${origin}/stats`)).json()) as { total: number; range206: number; bytesSent: number };
        const uploadFrames = loader.uploads.frames;
        const uploadTotalMs = uploadFrames.reduce((s, f) => s + f.ms, 0);
        const maxFrameBytes = uploadFrames.reduce((m, f) => Math.max(m, f.bytes), 0);
        const cacheStats = loader.stats();
        cache.close();
        return {
          label,
          totalMs: Number(totalMs.toFixed(1)),
          downloadSpanMs: Number(Math.max(0, downloadEndMs - t0).toFixed(1)),
          decodeSpanMs: Number(Math.max(0, decodeEndMs - t0).toFixed(1)),
          decodeWorkerSumMs: Number(decodeCpuSumMs.toFixed(1)),
          uploadMainThreadMs: Number(uploadTotalMs.toFixed(2)),
          loadedBytes,
          placeholders,
          longTasksCount: longTasks.length,
          maxLongTaskMs: longTasks.length ? Math.max(...longTasks.map((x) => x.durationMs)) : 0,
          longTasks,
          networkRequests: serverStats.total,
          networkBytesSent: serverStats.bytesSent,
          cacheMode: cacheStats.cacheMode,
          hitRate: Number(cacheStats.hitRate.toFixed(4)),
          opfsUsageBytes,
          poolSize,
          uploadFrames: uploadFrames.length,
          maxFrameBytes,
        };
      },
      { origin, meshIds: serverInfo.bench100mb.meshIds, texIds: serverInfo.bench100mb.texIds, ...MODS, label, clearBefore },
    );
  };

  const cold = await runPassInPage('cold_loopback', true);
  // Real browser reload between cold and warm start so memory is clean and OPFS persistence is genuinely tested.
  const warm = await runPassInPage('warm_loopback', false);

  // Also measure under 100 Mbps / 20 ms RTT CDN emulation (ARCHITECTURE 9.3 reference bandwidth) via CDP.
  const cdp = await context.newCDPSession(page);
  await cdp.send('Network.enable');
  await cdp.send('Network.emulateNetworkConditions', {
    offline: false,
    latency: 20,
    downloadThroughput: 12_500_000, // 100 Mbps
    uploadThroughput: 12_500_000,
  });
  let cold100: typeof cold;
  let warm100: typeof warm;
  try {
    cold100 = await runPassInPage('cold_100mbps', true);
    warm100 = await runPassInPage('warm_100mbps', false);
  } finally {
    await cdp.send('Network.emulateNetworkConditions', {
      offline: false,
      latency: 0,
      downloadThroughput: -1,
      uploadThroughput: -1,
    });
    await cdp.detach();
  }

  await page.evaluate(
    async ({ assetsPath }) => {
      const { opfsSyncBackend }: typeof import('../src/render/assets/index') = await import(assetsPath);
      const cleanup = await opfsSyncBackend({ dirName: 'frontline-assets-bench100mb' });
      await cleanup.clear();
      cleanup.close?.();
    },
    { assetsPath: MODS.assetsPath },
  );
  const loopbackReductionPct = Number((((cold.totalMs - warm.totalMs) / cold.totalMs) * 100).toFixed(1));
  const cdn100ReductionPct = Number((((cold100.totalMs - warm100.totalMs) / cold100.totalMs) * 100).toFixed(1));
  const bench = { cold, warm, loopbackReductionPct, cold100, warm100, cdn100ReductionPct };

  console.log('[asset-loader bench100mb]', JSON.stringify(bench));
  for (const pass of [cold, warm, cold100, warm100]) {
    expect(pass.placeholders).toBe(0);
    expect(pass.loadedBytes).toBeGreaterThanOrEqual(100 * 1024 * 1024);
    expect(pass.cacheMode).toBe('opfs');
    expect(pass.longTasksCount).toBe(0);
  }
  expect(cold.networkRequests).toBeGreaterThan(0);
  expect(warm.networkRequests).toBe(0);
  expect(warm.hitRate).toBe(1);
  expect(cold100.networkRequests).toBeGreaterThan(0);
  expect(warm100.networkRequests).toBe(0);
  expect(warm100.hitRate).toBe(1);
  expect(cdn100ReductionPct).toBeGreaterThanOrEqual(50);

  if (process.env.FRONTLINE_WRITE_ASSET_REPORT === '1') {
    const outPath = join(repoRoot, 'reports/m1-asset-loader.json');
    mkdirSync(join(repoRoot, 'reports'), { recursive: true });
    writeFileSync(
      outPath,
      JSON.stringify(
        {
          measured_at: new Date().toISOString(),
          hardware_note: 'Workstation EPYC 7402 / headless Chromium (PI-109, not RTX 3060 reference machine); upload stage measures UploadScheduler rAF pacing without GPU driver buffer creation.',
          pack_payload_bytes: serverInfo.bench100mb.totalPayloadBytes,
          pack_count: serverInfo.bench100mb.packCount,
          entries: { meshes: serverInfo.bench100mb.meshIds.length, textures: serverInfo.bench100mb.texIds.length },
          loopback: { cold, warm, warm_vs_cold_reduction_pct: loopbackReductionPct },
          cdn_100mbps_20ms: { cold: cold100, warm: warm100, warm_vs_cold_reduction_pct: cdn100ReductionPct },
        },
        null,
        2,
      ) + '\n',
    );
  }
});

test('M1-006 acceptance 2: OPFS and Cache API disabled -> cacheMode === memory and full load succeeds', async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, 'storage', {
      configurable: true,
      value: {
        getDirectory: () => Promise.reject(new DOMException('OPFS disabled', 'SecurityError')),
        estimate: () => Promise.reject(new DOMException('Storage estimate disabled', 'SecurityError')),
      },
    });
    Object.defineProperty(window, 'caches', { configurable: true, value: undefined });
  });
  await openProbePage(page);
  const origin = `http://localhost:${serverInfo.port}`;
  const res = await page.evaluate(
    async ({ origin, texId, assetsPath, workerDecoderPath }) => {
      const { AssetLoader, CacheLayer, detectBackends, isPlaceholder }: typeof import('../src/render/assets/index') = await import(assetsPath);
      const { workerDecoder }: typeof import('../src/render/assets/worker-decoder') = await import(workerDecoderPath);
      const backends = await detectBackends('0.1.0-mem');
      const decoder = workerDecoder(2, undefined, { caps: { bc7: true }, basisBase: '/basis' });
      const loader = new AssetLoader({ base: `${origin}/range/demo`, cache: new CacheLayer({ backends }), decoder, log: () => undefined });
      await loader.init({ manifestUrl: `${origin}/range/demo/manifest.json`, tier: 'medium' });
      let pumping = true;
      const pump = () => {
        loader.uploads.runFrame();
        if (pumping) requestAnimationFrame(pump);
      };
      requestAnimationFrame(pump);
      try {
        const mesh = await loader.loadMesh('prop_demo_crate', 0);
        const tex = await loader.loadTexture(texId);
        const audio = await loader.loadAudioEntry('snd_demo_shot@opus');
        return {
          backendsCount: backends.length,
          cacheMode: loader.stats().cacheMode,
          meshOk: !isPlaceholder(mesh),
          texOk: !isPlaceholder(tex),
          audioOk: !('silent' in audio),
        };
      } finally {
        pumping = false;
        decoder.terminate();
      }
    },
    { origin, texId: serverInfo.demo.textures[0]!, ...MODS },
  );

  expect(res.backendsCount).toBe(0);
  expect(res.cacheMode).toBe('memory');
  expect(res.meshOk).toBe(true);
  expect(res.texOk).toBe(true);
  expect(res.audioOk).toBe(true);
});

test('M1-006 acceptance 3: 200 small entries upload frame budget P95 <= 1.5 ms and max bytes <= 2 MB', async ({ page }) => {
  await openProbePage(page);
  const origin = `http://localhost:${serverInfo.port}`;
  const res = await page.evaluate(
    async ({ origin, meshIds, assetsPath, workerDecoderPath }) => {
      const { AssetLoader, CacheLayer, isPlaceholder }: typeof import('../src/render/assets/index') = await import(assetsPath);
      const { workerDecoder }: typeof import('../src/render/assets/worker-decoder') = await import(workerDecoderPath);
      const decoder = workerDecoder(2);
      const loader = new AssetLoader({ base: `${origin}/range/upload200`, cache: new CacheLayer(), decoder, log: () => undefined });
      await loader.init({ manifestUrl: `${origin}/range/upload200/manifest.json`, tier: 'medium' });

      let pumping = true;
      const pump = () => {
        loader.uploads.runFrame();
        if (pumping) requestAnimationFrame(pump);
      };
      requestAnimationFrame(pump);

      let placeholders = 0;
      try {
        await Promise.all(
          meshIds.map((id) =>
            loader.loadMesh(id, 0).then((r) => {
              if (isPlaceholder(r)) placeholders++;
            }),
          ),
        );
      } finally {
        pumping = false;
        decoder.terminate();
      }
      const frames = loader.uploads.frames;
      const sortedMs = frames.map((f) => f.ms).sort((a, b) => a - b);
      const p50Ms = sortedMs[Math.floor(sortedMs.length * 0.5)] ?? 0;
      const p95Ms = sortedMs[Math.min(sortedMs.length - 1, Math.floor(sortedMs.length * 0.95))] ?? 0;
      const maxMs = sortedMs[sortedMs.length - 1] ?? 0;
      const maxBytes = frames.reduce((m, f) => Math.max(m, f.bytes), 0);
      return {
        count: meshIds.length,
        placeholders,
        frames: frames.length,
        p50Ms: Number(p50Ms.toFixed(3)),
        p95Ms: Number(p95Ms.toFixed(3)),
        maxMs: Number(maxMs.toFixed(3)),
        maxBytes,
      };
    },
    { origin, meshIds: serverInfo.upload200.meshIds, ...MODS },
  );

  console.log('[asset-loader upload200]', JSON.stringify(res));
  expect(res.placeholders).toBe(0);
  expect(res.frames).toBeGreaterThan(0);
  expect(res.p95Ms).toBeLessThanOrEqual(1.5);
  expect(res.maxBytes).toBeLessThanOrEqual(2 * 1024 * 1024);

  if (process.env.FRONTLINE_WRITE_ASSET_REPORT === '1') {
    const outPath = join(repoRoot, 'reports/m1-asset-loader.json');
    const cur = JSON.parse(readFileSync(outPath, 'utf8')) as Record<string, unknown>;
    cur.upload_200_small = res;
    writeFileSync(outPath, JSON.stringify(cur, null, 2) + '\n');
  }
});
