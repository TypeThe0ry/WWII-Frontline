import { expect, test } from '@playwright/test';

test.use({ launchOptions: { executablePath: process.env.FRONTLINE_CHROME, args: ['--disable-gpu'] } });

test.beforeEach(async ({ page }) => {
  await page.goto('/ui-gallery');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Component gallery');
});

test('Tab order, roving tabs, and keyboard edits survive locale and panel changes', async ({ page }) => {
  const keys = page.keyboard;
  const language = page.getByRole('combobox');
  await keys.press('Tab');
  await expect(language).toBeFocused();
  await keys.press('ArrowDown');
  await expect(page.locator('html')).toHaveAttribute('lang', 'zh-Hans');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('组件画廊');
  await keys.press('Tab');
  const controls = page.getByRole('tab', { name: '控件' });
  await expect(controls).toBeFocused();
  await keys.press('Tab');
  await expect(page.getByRole('tabpanel', { name: '控件' })).toBeFocused();
  await keys.press('Tab');
  await expect(page.getByRole('textbox')).toBeFocused();
  await keys.type('Ada');
  await keys.press('Tab');
  await expect(page.getByRole('slider')).toBeFocused();
  await keys.press('ArrowRight');
  await keys.press('Tab');
  await keys.press('Space');
  await expect(page.getByRole('checkbox')).toBeChecked();
  await expect(page.getByRole('status')).toHaveText('通知');
  await keys.press('Tab');
  await keys.press('ArrowRight');
  await expect(page.getByRole('radio', { name: '读数' })).toBeChecked();

  await controls.focus();
  await keys.press('ArrowLeft'); // wraps from the first to last tab
  const readouts = page.getByRole('tab', { name: '读数' });
  await expect(readouts).toBeFocused();
  await expect(readouts).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('meter')).toHaveJSProperty('value', 51);
  await expect(page.getByRole('progressbar')).toHaveCount(2);
  await expect(page.getByRole('progressbar').first()).toHaveJSProperty('value', 51);
  await keys.press('Home');
  await expect(controls).toBeFocused();
  await expect(page.getByRole('textbox')).toHaveValue('Ada');
});

test('modal contains forward and reverse Tab, restores its opener on Escape and confirmation', async ({ page }) => {
  const preview = page.getByRole('button', { name: 'Preview', exact: true });
  const dialog = page.getByRole('dialog', { name: 'Confirm preview' });
  await preview.focus();
  await expect(page.getByRole('tooltip')).toHaveText('Confirm preview');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('tooltip')).toHaveCount(0);
  await page.keyboard.press('Enter');
  const cancel = dialog.getByRole('button', { name: 'Cancel' });
  const ok = dialog.getByRole('button', { name: 'OK' });
  await expect(cancel).toBeFocused();
  await page.keyboard.press('Shift+Tab');
  await expect(ok).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(cancel).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  await expect(preview).toBeFocused();
  await page.keyboard.press('Space');
  await expect(cancel).toBeFocused();
  await page.keyboard.press('Tab');
  await page.keyboard.press('Enter');
  await expect(preview).toBeFocused();
  await expect(page.getByRole('status')).toHaveText('Ready');
  await page.keyboard.press('Tab');
  const join = page.getByRole('button', { name: 'Join', exact: true });
  await expect(join).toBeFocused();
  await expect(join).toBeDisabled();
  await expect(join).toHaveAccessibleDescription(/full/i);
});

test('virtual list exposes absolute positions after scrolling to the last row', async ({ page }) => {
  await page.getByRole('tab', { name: 'Readouts' }).click();
  const list = page.getByRole('list', { name: 'Entries' });
  await list.focus();
  await expect(list.getByRole('listitem').first()).toHaveAttribute('aria-posinset', '1');
  await page.keyboard.press('End');
  await expect(list.getByRole('listitem').last()).toHaveText('Entry 1000');
  await expect(list.getByRole('listitem').last()).toHaveAttribute('aria-setsize', '1000');
  expect(await list.getByRole('listitem').count()).toBeLessThan(20);
});

test('missing optional font bundle flags fallback without blocking the gallery', async ({ page }) => {
  await page.route('**/fonts/fonts.css', route => route.fulfill({ status: 404, body: '' }));
  await page.goto('/ui-gallery?fonts=1&lang=zh-Hans');
  await expect(page.locator('html')).toHaveAttribute('data-font-fallback', '');
  await expect(page.getByRole('tab', { name: '控件' })).toBeEnabled();
});

// Review artifacts, not aesthetic assertions or approved visual baselines.
test('unstyled controls and readouts are reachable in both languages at both review scales', async ({ page }, info) => {
  for (const [lang, controls, readouts] of [['en', 'Controls', 'Readouts'], ['zh-Hans', '控件', '读数']] as const) {
    await page.goto(`/ui-gallery?lang=${lang}`);
    for (const scale of [1, 1.5]) {
      await page.evaluate(value => { document.documentElement.style.zoom = String(value); }, scale);
      for (const [panel, name] of [['controls', controls], ['readouts', readouts]] as const) {
        await page.getByRole('tab', { name }).click();
        await expect(page.getByRole('tabpanel', { name })).toBeVisible();
        const filename = `${lang}-${scale}-${panel}.png`;
        const path = info.outputPath(filename);
        await page.screenshot({ fullPage: true, path });
        await info.attach(filename, { path, contentType: 'image/png' });
      }
    }
  }
});
