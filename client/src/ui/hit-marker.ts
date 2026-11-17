/**
 * Hit marker and damage direction bars (M0-025; the full hit indication of UI_SPEC is M1-057), pure DOM.
 *  - `hit.confirm` (server-confirmed, shooter only): an X in the screen centre for `HIT_MARKER_MS`.
 *  - `damage.taken`: a red bar on the screen edge towards the damage source (4 directions) for `HIT_MARKER_MS`.
 * The handler writes the style synchronously, so the marker is visible in the next frame that is painted.
 * The client never shows predicted damage or kills (ARCHITECTURE 7.2).
 */
import type { EventBus, GameEvents } from '../game/event-bus';

export const HIT_MARKER_MS = 120;

/** Which edge a damage source at `angle` (rad, 0 = ahead, clockwise) maps to: 0 top, 1 right, 2 bottom, 3 left. */
export function damageEdge(angle: number): 0 | 1 | 2 | 3 {
  const q = Math.floor(((angle % (Math.PI * 2)) + Math.PI * 2 + Math.PI / 4) / (Math.PI / 2)) % 4;
  return q as 0 | 1 | 2 | 3;
}

export interface HitMarker {
  marker: HTMLElement;
  bars: HTMLElement[];
  dispose(): void;
}

const BAR_CSS = [
  'left:20%;right:20%;top:0;height:14px;',
  'top:20%;bottom:20%;right:0;width:14px;',
  'left:20%;right:20%;bottom:0;height:14px;',
  'top:20%;bottom:20%;left:0;width:14px;',
];

export function createHitMarker(parent: HTMLElement, bus: EventBus<GameEvents>): HitMarker {
  const doc = parent.ownerDocument;
  const marker = doc.createElement('div');
  marker.id = 'hit-marker';
  marker.setAttribute('aria-hidden', 'true');
  marker.style.cssText = 'position:fixed;left:50%;top:50%;width:0;height:0;pointer-events:none;z-index:21;opacity:0;';
  for (const rot of [45, -45]) {
    const b = doc.createElement('div');
    b.style.cssText = `position:absolute;left:-1px;top:-11px;width:2px;height:22px;background:#fff;box-shadow:0 0 0 1px rgba(0,0,0,.6);transform:rotate(${rot}deg);`;
    marker.appendChild(b);
  }
  parent.appendChild(marker);
  const bars = BAR_CSS.map((css, i) => {
    const b = doc.createElement('div');
    b.id = `damage-bar-${i}`;
    b.setAttribute('aria-hidden', 'true');
    b.style.cssText = `position:fixed;${css}pointer-events:none;z-index:21;opacity:0;background:linear-gradient(${['to bottom', 'to left', 'to top', 'to right'][i]},rgba(200,20,20,.85),rgba(200,20,20,0));`;
    parent.appendChild(b);
    return b;
  });
  const timers = new Map<HTMLElement, ReturnType<typeof setTimeout>>();
  const flash = (el: HTMLElement): void => {
    el.style.opacity = '1';
    el.dataset.visible = '1';
    const t = timers.get(el);
    if (t !== undefined) clearTimeout(t);
    timers.set(
      el,
      setTimeout(() => {
        el.style.opacity = '0';
        el.dataset.visible = '0';
        timers.delete(el);
      }, HIT_MARKER_MS),
    );
  };
  const offs = [
    bus.on('hit.confirm', () => flash(marker)),
    bus.on('damage.taken', (e) => flash(bars[damageEdge(e.angle)] as HTMLElement)),
  ];
  return {
    marker,
    bars,
    dispose: () => {
      for (const off of offs) off();
      for (const t of timers.values()) clearTimeout(t);
      marker.remove();
      for (const b of bars) b.remove();
    },
  };
}
