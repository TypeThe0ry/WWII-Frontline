import { expect, test, type Page, type TestInfo } from '@playwright/test';

test.use({ viewport: { width: 1920, height: 1080 }, launchOptions: {
  executablePath: process.env.FRONTLINE_CHROME, args: ['--disable-gpu'],
} });

async function record(info: TestInfo, name: string, metrics: Record<string, unknown>) {
  const json = JSON.stringify(metrics);
  console.log(`[ui-perf:${name}] ${json}`);
  await info.attach(name, { body: json, contentType: 'application/json' });
}

type TraceEvent = { name: string; ts: number; pid: number; tid: number };

/** Count renderer main-thread Paint events (not raster/composite or DOM mutations).
 * Both boundaries are page marks, so CDP delivery latency does not alter the window. */
async function paintWindow(page: Page) {
  const cdp = await page.context().newCDPSession(page);
  const events: TraceEvent[] = [];
  cdp.on('Tracing.dataCollected', ({ value }) => {
    for (const event of value) {
      if (event.name === 'Paint' || event.name?.startsWith('ui-perf:')) {
        events.push({ name: event.name, ts: Number(event.ts), pid: Number(event.pid), tid: Number(event.tid) });
      }
    }
  });
  try {
    await cdp.send('Tracing.start', { categories: 'devtools.timeline,blink.user_timing', transferMode: 'ReportEvents' });
    await page.evaluate(async () => {
      const frames = async (count: number) => {
        for (let i = 0; i < count; i++) await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
      };
      await document.fonts.ready;
      await frames(6);
      performance.mark('ui-perf:start');
      await frames(60);
      performance.mark('ui-perf:idle-end');
      // A native modal is in the top layer even if the measured root is empty or fixed.
      const probe = document.createElement('dialog');
      document.body.append(probe);
      probe.textContent = 'Paint control';
      probe.showModal();
      await frames(4);
      performance.mark('ui-perf:end');
      probe.remove();
    });
    const complete = new Promise<boolean>(resolve => cdp.once('Tracing.tracingComplete', e => resolve(!e.dataLossOccurred)));
    await cdp.send('Tracing.end');
    expect(await complete, 'Chromium dropped trace data').toBe(true);
  } finally {
    await cdp.detach();
  }
  const [start, idleEnd, end] = ['ui-perf:start', 'ui-perf:idle-end', 'ui-perf:end'].map(name => {
    const marks = events.filter(event => event.name === name);
    expect(marks, `unique renderer mark ${name}`).toHaveLength(1);
    return marks[0]!;
  }) as [TraceEvent, TraceEvent, TraceEvent];
  const paints = events.filter(e => e.name === 'Paint' && e.pid === start.pid && e.tid === start.tid);
  const count = (a: number, b: number) => paints.filter(e => a <= e.ts && e.ts < b).length;
  return { idleFrames: 60, idleMs: (idleEnd.ts - start.ts) / 1000,
    idlePaints: count(start.ts, idleEnd.ts), controlPaints: count(idleEnd.ts, end.ts) };
}

async function checkIdle(page: Page, info: TestInfo, scene: string, nodes: number, budget: number) {
  const trace = await paintWindow(page);
  await record(info, scene, { nodes, ...trace });
  expect(nodes).toBeLessThanOrEqual(budget);
  expect(trace.controlPaints, 'visible paint control must register on the same renderer thread').toBeGreaterThan(0);
  expect(trace.idlePaints, 'settled scene should not continuously repaint').toBe(0);
}

test('settled gallery: DOM and Chromium repaint budgets', async ({ page }, info) => {
  await page.goto('/ui-gallery');
  const root = page.locator('[data-ui-gallery]');
  await expect(root.getByRole('button').first()).toBeVisible();
  await expect(page.locator('[data-uiperf]')).toHaveCount(0);
  await checkIdle(page, info, 'gallery', await root.locator('*').count(), 1500);
  await root.getByRole('tab', { name: 'Readouts' }).click();
  const list = root.getByRole('list', { name: 'Entries' });
  await list.evaluate(el => { el.scrollTop = el.scrollHeight / 2; });
  await expect(list.locator('[aria-posinset="501"]')).toBeVisible();
  await checkIdle(page, info, 'gallery-readouts-mid-list', await root.locator('*').count(), 1500);
});

test('real HUD: populated snapshot and no-content host budgets', async ({ page }, info) => {
  await page.goto('/src/ui/perf/hud-fixture.html');
  const host = page.locator('[data-hud-host]');
  const hud = host.locator('[data-hud]');
  await expect(hud.locator('[data-member-id]')).toHaveCount(8);
  await expect(hud.locator('[data-hud-part="ammo"]')).toBeVisible();
  await checkIdle(page, info, 'hud', await hud.locator('*').count(), 350);
  await host.dispatchEvent('perf:spectate');
  await expect(hud).toHaveCount(0);
  await expect(host).toBeAttached();
  await checkIdle(page, info, 'empty-hud-including-host', 1 + await host.locator('*').count(), 20);
});

test('gallery ?uiperf=1: 100 committed Preact signal changes per frame', async ({ page }, info) => {
  await page.goto('/ui-gallery?uiperf=1');
  const probe = page.locator('[data-ui-gallery] [data-uiperf]');
  await expect(probe).toHaveAttribute('data-uiperf', /done|error/, { timeout: 20_000 });
  expect(await probe.getAttribute('data-uiperf'), await probe.textContent() ?? '').toBe('done');
  const metrics = await probe.evaluate(element => Object.fromEntries(Object.entries((element as HTMLElement).dataset)
    .filter(([key]) => key !== 'uiperf').map(([key, value]) => [key, Number(value)])));
  await record(info, 'signals', metrics);
  expect(metrics).toMatchObject({ signals: 100, warmup: 30, frames: 120 });
  expect(metrics.nodes).toBe(await page.locator('[data-ui-gallery] *').count());
  expect(metrics.meanMs).toBeGreaterThan(0);
  expect(metrics.meanMs).toBeLessThanOrEqual(0.8);
  expect(metrics.p95Ms).toBeGreaterThanOrEqual(0);
});
