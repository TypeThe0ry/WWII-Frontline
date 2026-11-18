import { describe, it, expect, beforeEach } from 'vitest';
import en from './i18n/en.json';
import zh from './i18n/zh-Hans.json';
import { t, setLocale, registerMessages, missingKeys, format } from './i18n/index';
import { checkCatalogs, placeholders } from './i18n/check';
import { uiScale, breakpoint, scaledFontPx, safeBox } from './ui-scale';
import { virtualWindow } from './virtual-list';
import { uiScreen, SCREENS } from './screens';
import { readFileSync } from 'node:fs';
const css = readFileSync(new URL('./tokens/plotline.css', import.meta.url).pathname, 'utf8');

beforeEach(() => {
  registerMessages('en', en);
  registerMessages('zh-Hans', zh);
  setLocale('en');
});

describe('i18n', () => {
  it('interpolates, pluralises, switches at runtime', () => {
    expect(t('menu.play')).toBe('Play');
    expect(t('hud.ammo.reserve', { n: 1 })).toBe('1 magazine');
    expect(t('hud.ammo.reserve', { n: 3 })).toBe('3 magazines');
    expect(t('common.disabledReason', { reason: 'no ammo' })).toBe('Unavailable: no ammo');
    setLocale('zh-Hans');
    expect(t('menu.play')).toBe('开始游戏');
    expect(t('hud.ammo.reserve', { n: 1 })).toBe('1 个弹匣');
  });
  it('missing key shows ⟦key⟧ and is recorded', () => {
    expect(t('no.such')).toBe('⟦no.such⟧');
    expect(missingKeys.has('no.such')).toBe(true);
  });
  it('format keeps unknown params and exact plural arms', () => {
    expect(format('en', 'a {x} b', {})).toBe('a {x} b');
    expect(format('en', '{n, plural, =0 {none} one {# x} other {# xs}}', { n: 0 })).toBe('none');
  });
  it('catalogs are consistent; check detects missing/extra/placeholder issues', () => {
    expect(checkCatalogs(en, zh)).toEqual([]);
    expect(placeholders(en['hud.ammo.reserve'])).toEqual(['n']);
    const z = { ...zh } as Record<string, string>;
    delete z['menu.play'];
    z['x.extra'] = 'x';
    z['hud.players'] = '{m} 名玩家';
    const kinds = checkCatalogs(en, z).map((i) => i.kind).sort();
    expect(kinds).toEqual(['extra', 'missing', 'placeholder']);
  });
});

describe('ui scale', () => {
  it('base, clamp, large cap, user scale', () => {
    expect(uiScale(1920, 1080)).toBeCloseTo(1);
    expect(uiScale(1280, 720)).toBeCloseTo(0.667, 3);
    expect(uiScale(800, 450)).toBeCloseTo(0.667, 3);
    expect(uiScale(3840, 2160)).toBe(1.5);
    expect(uiScale(1920, 1080, 1.5)).toBeCloseTo(1.5);
  });
  it('breakpoints', () => {
    expect(breakpoint(1280, 720)).toBe('compact');
    expect(breakpoint(1920, 1080)).toBe('standard');
    expect(breakpoint(2560, 1440)).toBe('large');
    expect(breakpoint(3440, 1440)).toBe('ultrawide');
  });
  it('font floor and safe box', () => {
    expect(scaledFontPx(12, 0.667, false)).toBe(11);
    expect(scaledFontPx(12, 0.667, true)).toBe(12);
    expect(safeBox(1920, 1080, 1)).toEqual({ x: 48, y: 32.4 });
  });
});

describe('virtual list', () => {
  it('window covers viewport with overscan and padding adds up', () => {
    const w = virtualWindow(1000, 20, 200, 2000);
    expect(w.start).toBe(97);
    expect(w.end).toBe(100 + 11 + 3);
    expect(w.padTop + (w.end - w.start) * 20 + w.padBottom).toBe(1000 * 20);
    expect(virtualWindow(0, 20, 200, 0).end).toBe(0);
    expect(virtualWindow(5, 20, 200, 99999).end).toBe(5);
  });
});

describe('tokens / screens', () => {
  it('tokens include the spec colours and z-index ladder', () => {
    for (const v of ['--pl-accent: #e4cc8a', '--pl-friend: #3da5ff', '--pl-enemy: #ff7a2f', '--pl-s10: 96px', '--pl-z-pointer-lock: 80']) expect(css).toContain(v);
  });
  it('screen signal', () => {
    expect(SCREENS).toHaveLength(9);
    uiScreen.value = 'menu';
    expect(uiScreen.value).toBe('menu');
  });
});
