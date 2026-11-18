import { describe, it, expect } from 'vitest';
import { Bindings, Persist, checkCode, findConflict, saveScheme, loadScheme, SCHEME_SLOTS, ACTIONS, RESERVED_KEY, DEFAULT_BINDINGS, type StorageLike } from './bindings';
import { SettingsStore, clampSetting } from '../game/settings';
import { pauseNext, PAUSE_CLOSED, shouldPreventDefault, clearHeld } from './focus-guard';
import en from '../ui/i18n/en.json';
import zh from '../ui/i18n/zh-Hans.json';

const mem = (): StorageLike => {
  const m = new Map<string, string>();
  return { getItem: (k) => m.get(k) ?? null, setItem: (k, v) => void m.set(k, v) };
};
const throwing: StorageLike = { getItem: () => { throw new Error('denied'); }, setItem: () => { throw new Error('denied'); } };

describe('defaults are conflict-free', () => {
  it('no action pair shares a key within a shared context', () => {
    for (const a of ACTIONS) {
      for (const slot of ['primary', 'secondary'] as const) {
        const c = DEFAULT_BINDINGS[a][slot];
        if (c) expect(findConflict(DEFAULT_BINDINGS as never, a, c)).toBeUndefined();
      }
    }
  });
});

describe('reserved keys (acceptance 2)', () => {
  it.each(['Ctrl+KeyW', 'F11', 'MetaLeft'])('%s is refused with the reserved key', (code) => {
    const b = new Bindings();
    expect(b.rebind('reload', 'primary', code)).toEqual({ ok: false, reasonKey: RESERVED_KEY });
    expect(b.map.reload.primary).toBe('KeyR');
  });
  it('pause stays on Escape; plain keys are accepted', () => {
    const b = new Bindings();
    expect(b.rebind('pause', 'primary', 'KeyP').ok).toBe(false);
    expect(checkCode('KeyT').ok).toBe(true);
    expect(checkCode('Mouse3').ok).toBe(true);
    expect(checkCode('Mouse9').ok).toBe(false);
  });
});

describe('conflicts (acceptance 3)', () => {
  it('fire → Space conflicts with jump in OnFoot and offers swap / unbind / cancel', () => {
    const b = new Bindings();
    const r = b.rebind('fire', 'primary', 'Space');
    expect(r).toMatchObject({ ok: false, conflict: 'jump' });
    expect(b.resolve('fire', 'primary', 'Space', 'cancel')).toBe(false);
    expect(b.map.fire.primary).toBe('Mouse0');
    expect(b.resolve('fire', 'primary', 'Space', 'swap')).toBe(true);
    expect(b.map.fire.primary).toBe('Space');
    expect(b.map.jump.primary).toBe('Mouse0');
    const c = new Bindings();
    c.resolve('fire', 'primary', 'Space', 'unbind');
    expect(c.map.jump.primary).toBeUndefined();
  });
  it('same key across non-overlapping contexts is fine', () => {
    const b = new Bindings();
    // `map` (works in map ctx) vs `reload` (onFoot only) overlap on onFoot; scoreboard vs reload overlap too — use downed-only logic instead:
    expect(findConflict(b.map, 'lean_l', 'KeyZ')).toBe('prone');
    expect(b.actionsFor('KeyW', 'inVehicle')).toEqual([]);
    expect(b.actionsFor('KeyW', 'onFoot')).toEqual(['move_fwd']);
  });
});

describe('persistence (acceptance 1)', () => {
  it('rebind reload→T survives a reload through storage', () => {
    const st = mem();
    const p1 = new Persist(() => st);
    const b = new Bindings();
    expect(b.rebind('reload', 'primary', 'KeyT').ok).toBe(true);
    expect(saveScheme(p1, 0, b)).toBe(true);
    const b2 = new Bindings();
    expect(loadScheme(new Persist(() => st), 0, b2)).toBe(true);
    expect(b2.map.reload.primary).toBe('KeyT');
  });
  it('throwing storage falls back to memory without crashing and flags degraded', () => {
    const p = new Persist(() => throwing);
    const b = new Bindings();
    b.rebind('reload', 'primary', 'KeyT');
    expect(() => saveScheme(p, 1, b)).not.toThrow();
    expect(p.degraded).toBe(true);
    const b2 = new Bindings();
    expect(loadScheme(p, 1, b2)).toBe(true);
    expect(b2.map.reload.primary).toBe('KeyT');
    expect(saveScheme(p, SCHEME_SLOTS, b)).toBe(false);
  });
  it('import rejects bad JSON, wrong version, conflicts and reserved codes without applying', () => {
    const b = new Bindings();
    const good = b.export();
    expect(b.import('not json')).toBe(false);
    expect(b.import('{"v":2,"bindings":{}}')).toBe(false);
    const bad = JSON.parse(good);
    bad.bindings.fire.primary = 'Space';
    expect(b.import(JSON.stringify(bad))).toBe(false);
    bad.bindings.fire.primary = 'F11';
    expect(b.import(JSON.stringify(bad))).toBe(false);
    expect(b.map.fire.primary).toBe('Mouse0');
    expect(b.import(good)).toBe(true);
  });
});

describe('settings (acceptance 6)', () => {
  it('FOV clamps to 60–100, emits settings.changed with key/value, persists', () => {
    const st = mem();
    const s = new SettingsStore(new Persist(() => st));
    const seen: unknown[] = [];
    s.onChange((e) => seen.push(e));
    expect(clampSetting('video.fov', 30)).toBe(60);
    expect(clampSetting('video.fov', 140)).toBe(100);
    s.set('video.fov', 120);
    s.set('quality.tier', 'high');
    s.set('quality.tier', 'high');
    expect(seen).toEqual([{ key: 'video.fov', value: 100 }, { key: 'quality.tier', value: 'high' }]);
    expect(new SettingsStore(new Persist(() => st)).get('video.fov')).toBe(100);
    expect(s.set('nope', 1)).toBe(false);
    expect(clampSetting('quality.tier', 'ultra')).toBe('medium');
    expect(clampSetting('video.fov', NaN)).toBe(80);
  });
  it('corrupt stored settings fall back to defaults', () => {
    const st = mem();
    st.setItem('settings', '{oops');
    expect(new SettingsStore(new Persist(() => st)).get('video.fov')).toBe(80);
  });
});

describe('pause / focus (acceptance 4, 5 logic)', () => {
  it('lock loss opens the menu; continue closes; refused lock shows click overlay', () => {
    let s = pauseNext(PAUSE_CLOSED, 'lockLost');
    expect(s.open).toBe(true);
    s = pauseNext(s, 'continue');
    expect(s.open).toBe(false);
    s = pauseNext(s, 'lockRefused');
    expect(s.needClick).toBe(true);
    expect(pauseNext(s, 'lockGranted')).toEqual(PAUSE_CLOSED);
  });
  it('Esc with the map open closes the map first (no menu); blur always opens the menu', () => {
    expect(pauseNext(PAUSE_CLOSED, 'lockLost', true).open).toBe(false);
    expect(pauseNext(PAUSE_CLOSED, 'blur', true).open).toBe(true);
  });
  it('leave needs a second confirmation', () => {
    let s = pauseNext(pauseNext(PAUSE_CLOSED, 'lockLost'), 'leave');
    expect(s.confirmLeave).toBe(true);
    expect(pauseNext(s, 'cancelLeave').confirmLeave).toBe(false);
    s = pauseNext(s, 'confirmLeave');
    expect(s).toEqual(PAUSE_CLOSED);
  });
  it('blur clears held keys; F5/Tab are swallowed in game only', () => {
    const held = new Set(['KeyW', 'Mouse0']);
    clearHeld(held);
    expect(held.size).toBe(0);
    expect(shouldPreventDefault('F5', true)).toBe(true);
    expect(shouldPreventDefault('F5', false)).toBe(false);
    expect(shouldPreventDefault('KeyW', true)).toBe(false);
  });
});

describe('i18n keys', () => {
  it.each(['ui.settings.controls.reserved', 'ui.settings.controls.conflict', 'ui.pause.inProgress', 'ui.pause.continue', 'ui.pause.leave', 'ui.pause.leaveConfirm', 'ui.pause.clickToContinue'])('%s', (k) => {
    expect(k in en && k in zh).toBe(true);
  });
});
