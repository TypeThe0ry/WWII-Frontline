import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { EventBus, type GameEvents } from '../game/event-bus';
import { createCrosshair, CROSSHAIR_SPREAD_MS } from './crosshair';
import { createHitMarker, damageEdge, HIT_MARKER_MS } from './hit-marker';

// Minimal DOM stand-in (no jsdom in this repo): enough for the two overlays.
class FakeEl {
  children: FakeEl[] = [];
  style: Record<string, string> & { cssText: string; setProperty(k: string, v: string): void };
  dataset: Record<string, string> = {};
  attrs: Record<string, string> = {};
  id = '';
  removed = false;
  ownerDocument: { createElement(): FakeEl };
  constructor(doc: { createElement(): FakeEl }) {
    this.ownerDocument = doc;
    const vars: Record<string, string> = {};
    this.style = Object.assign(Object.create(null) as Record<string, string>, { cssText: '', setProperty: (k: string, v: string) => (vars[k] = v) }) as FakeEl['style'];
    this.vars = vars;
  }
  vars: Record<string, string>;
  appendChild(c: FakeEl): void {
    this.children.push(c);
  }
  setAttribute(k: string, v: string): void {
    this.attrs[k] = v;
  }
  remove(): void {
    this.removed = true;
  }
}
const doc: { createElement(): FakeEl } = { createElement: () => new FakeEl(doc) };
const shown = (e: HTMLElement): boolean => e.style.opacity === '1'; // a real element would parse `opacity:0` from cssText
const root = () => new FakeEl(doc) as unknown as HTMLElement;

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe('hit marker', () => {
  it('hit.confirm shows the marker synchronously (visible in the next painted frame) and hides it after 120 ms', () => {
    const bus = new EventBus<GameEvents>();
    const hm = createHitMarker(root(), bus);
    expect(shown(hm.marker)).toBe(false);
    bus.emit('hit.confirm', { target: 3, zone: 0, damage: 0.4, killed: false });
    expect(shown(hm.marker)).toBe(true); // set inside the emit call: no frame of latency
    vi.advanceTimersByTime(HIT_MARKER_MS - 1);
    expect(shown(hm.marker)).toBe(true);
    vi.advanceTimersByTime(2);
    expect(shown(hm.marker)).toBe(false);
    expect(HIT_MARKER_MS).toBe(120);
  });

  it('a second hit inside the window restarts the 120 ms', () => {
    const bus = new EventBus<GameEvents>();
    const hm = createHitMarker(root(), bus);
    bus.emit('hit.confirm', { target: 1, zone: 0, damage: 0.1, killed: false });
    vi.advanceTimersByTime(100);
    bus.emit('hit.confirm', { target: 1, zone: 0, damage: 0.1, killed: false });
    vi.advanceTimersByTime(100);
    expect(shown(hm.marker)).toBe(true);
    vi.advanceTimersByTime(30);
    expect(shown(hm.marker)).toBe(false);
  });

  it('damage.taken lights the edge bar towards the source: ahead -> top, right -> right, behind -> bottom, left -> left', () => {
    expect([0, Math.PI / 2, Math.PI, (3 * Math.PI) / 2, -Math.PI / 2, 0.7, 2.3, Math.PI * 2 - 0.1].map(damageEdge)).toEqual([0, 1, 2, 3, 3, 0, 1, 0]);
    const bus = new EventBus<GameEvents>();
    const hm = createHitMarker(root(), bus);
    bus.emit('damage.taken', { angle: Math.PI, amount: 0.5 });
    expect(hm.bars.map(shown)).toEqual([false, false, true, false]);
    vi.advanceTimersByTime(HIT_MARKER_MS + 1);
    expect(hm.bars.map(shown)).toEqual([false, false, false, false]);
  });

  it('dispose unsubscribes', () => {
    const bus = new EventBus<GameEvents>();
    const hm = createHitMarker(root(), bus);
    hm.dispose();
    expect(bus.listenerCount('hit.confirm')).toBe(0);
    expect(bus.listenerCount('damage.taken')).toBe(0);
  });
});

describe('crosshair', () => {
  it('widens for CROSSHAIR_SPREAD_MS on fire.local and shows no text', () => {
    const bus = new EventBus<GameEvents>();
    const ch = createCrosshair(root(), bus);
    const el = ch.el as unknown as FakeEl;
    expect(el.children.length).toBe(5);
    bus.emit('fire.local', { weapon: 0, muzzlePos: { x: 0, y: 0, z: 0 }, dir: { x: 0, y: 0, z: 1 } });
    expect(el.vars['--fl-xh-gap']).toBe('11px');
    vi.advanceTimersByTime(CROSSHAIR_SPREAD_MS + 1);
    expect(el.vars['--fl-xh-gap']).toBe('6px');
    ch.dispose();
    expect(bus.listenerCount('fire.local')).toBe(0);
  });
});
