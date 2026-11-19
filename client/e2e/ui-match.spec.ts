import { expect, test } from '@playwright/test';

const FIXTURE_PAGE = '/e2e/ui-fixture.html';

test('Deploy sends C0 once, shows rejection and recommendation, and waits for authoritative spawn after tick 160', async ({ page }) => {
  await page.goto(FIXTURE_PAGE);
  const deploy = page.getByRole('button', { name: 'Deploy', exact: true });
  await expect(page.locator('canvas[aria-label="Deployment map"]')).toBeVisible();
  await deploy.click();
  await expect(deploy).toBeDisabled();
  await expect(page.getByText('That spawn point is blocked.')).toBeVisible();
  await expect(page.getByRole('status', { name: 'C0 packets sent' })).toHaveText('[[16,255,255,3,12,2]]');
  await page.getByRole('button', { name: /North garrison.*Recommended/ }).click();
  await deploy.click();
  await expect(page.getByText('Spawning in 1 s.')).toBeVisible();
  await expect(page.getByRole('status', { name: 'C0 packets sent' })).toHaveText('[[16,255,255,3,12,2],[16,0,4,3,12,2]]');
  await page.getByRole('button', { name: 'Advance spawn clock' }).click();
  await expect(page.getByText('Deployment accepted. Awaiting spawn confirmation.')).toBeVisible();
  await expect(deploy).toBeDisabled();
});

test('HUD snapshot and alive/downed/dead/respawn DOM transitions, without leaking killer data while downed', async ({ page }) => {
  await page.goto(FIXTURE_PAGE);
  await expect(page.getByLabel('Combat HUD')).toBeVisible();
  await page.getByRole('button', { name: 'Apply fixture snapshot' }).click();
  await expect(page.locator('[data-hud-part="crosshair"]')).toHaveAttribute('data-hit', /headshot|hit/);
  await page.getByRole('button', { name: 'Down fixture player' }).click();
  const life = page.locator('[data-downed-death]');
  await expect(life).toHaveAttribute('data-phase', 'downed');
  await expect(life.getByText(/Bleedout remaining: 80 s/)).toBeVisible();
  await expect(page.getByText('Fixture Opponent')).toHaveCount(0);
  await life.locator('[data-downed-action="call-medic"]').click();
  await expect(page.getByRole('status', { name: 'Medic call intents' })).toHaveText('1');
  await expect(life.locator('[data-downed-action="call-medic"]')).toBeDisabled();
  await page.getByRole('button', { name: 'Kill fixture player' }).click();
  await expect(life).toHaveAttribute('data-phase', 'dead');
  await expect(life.getByText(/Fixture Opponent/)).toBeVisible();
  await expect(life.getByText(/Respawn: 1 s/)).toBeVisible();
  await page.getByRole('button', { name: 'Respawn fixture player' }).click();
  await expect(life).toHaveCount(0);
  await expect(page.getByLabel('Combat HUD')).toBeVisible();
});

test('placement DOM blocks an invalid preview and displays the reply to one mock command', async ({ page }) => {
  await page.goto(FIXTURE_PAGE);
  const place = page.getByRole('button', { name: 'Place Garrison' });
  const preview = page.locator('[data-placement-pattern]');
  const intents = page.getByRole('status', { name: 'Placement intents' });
  await expect(page.getByRole('region', { name: 'Board Fixture vehicle' })).toContainText('Boarding unavailable');
  await page.getByRole('button', { name: 'Garrison kit' }).click();
  await expect(preview).toHaveAttribute('data-placement-pattern', 'hatched');
  await expect(place).toBeDisabled();
  await expect(intents).toHaveText('0');
  await page.getByRole('button', { name: 'Aim at valid ground' }).click();
  await expect(place).toBeEnabled();
  await place.click();
  await expect(intents).toHaveText('1');
  await expect(preview).toHaveAttribute('data-placement-pattern', 'dotted');
  await expect(place).toBeDisabled();
});
