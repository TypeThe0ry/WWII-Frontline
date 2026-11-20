import { expect, test } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import { basename, join } from 'node:path';

// Point at the result of the full, ledger-verified subset build. Do not put fonts in git.
const directory = process.env.FRONTLINE_UI_FONT_BUNDLE;
test.use({ launchOptions: { executablePath: process.env.FRONTLINE_CHROME, args: ['--disable-gpu'] } });

test('Chromium loads verified Latin and Chinese shards on demand', async ({ page }) => {
  test.skip(!directory, 'Generate the licensed full bundle and set FRONTLINE_UI_FONT_BUNDLE');
  const report = JSON.parse(await readFile(join(directory!, 'report.json'), 'utf8')) as {
    mode: string; files: { file: string; shard: string }[];
  };
  expect(report.mode).toBe('full');
  const shards = new Map(report.files.map(f => [f.file, f.shard]));
  const requested: string[] = [];
  await page.route(url => url.pathname.startsWith('/fonts/'), async route => {
    const file = basename(new URL(route.request().url()).pathname);
    if (file !== 'fonts.css' && !shards.has(file)) throw new Error(`Unexpected font request: ${file}`);
    if (shards.has(file)) requested.push(shards.get(file)!);
    await route.fulfill({ path: join(directory!, file), contentType: file === 'fonts.css' ? 'text/css' : 'font/woff2' });
  });
  await page.goto('/ui-gallery?fonts=1');
  await expect.poll(() => page.evaluate(() => document.fonts.size)).toBe(report.files.length);
  const load = (family: string, text: string) => page.evaluate(async ([f, s]) =>
    (await document.fonts.load(`500 16px "${f}"`, s)).map(face => face.status), [family, text] as const);
  expect(await load('Frontline UI Sans', 'Ready')).toEqual(['loaded']);
  expect(requested).toEqual(['latin']);
  expect(await load('Noto Sans SC', '加载')).toEqual(['loaded']);
  expect(requested).toEqual(['latin', 'cjk0']);
  expect(await load('Noto Sans SC', '龛')).toEqual(['loaded']);
  expect(requested).toHaveLength(3);
  expect(requested[2]).toMatch(/^cjk[123]$/);
  await expect(page.locator('html')).not.toHaveAttribute('data-font-fallback');
});
