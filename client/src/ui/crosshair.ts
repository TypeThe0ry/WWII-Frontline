/**
 * Minimal crosshair (M0-025; the final HUD is M1-057): four ticks and a dot, pure DOM. It widens briefly on every
 * `fire.local`. No text, so nothing to translate. Subscribes through the event bus only.
 */
import type { EventBus, GameEvents } from '../game/event-bus';

export const CROSSHAIR_SPREAD_MS = 90;

export interface Crosshair {
  el: HTMLElement;
  dispose(): void;
}

export function createCrosshair(parent: HTMLElement, bus: EventBus<GameEvents>): Crosshair {
  const doc = parent.ownerDocument;
  const el = doc.createElement('div');
  el.id = 'crosshair';
  el.setAttribute('aria-hidden', 'true');
  el.style.cssText = 'position:fixed;left:50%;top:50%;width:0;height:0;pointer-events:none;z-index:20;';
  const bar = (css: string): void => {
    const b = doc.createElement('div');
    b.style.cssText = `position:absolute;background:#fff;box-shadow:0 0 0 1px rgba(0,0,0,.55);${css}`;
    el.appendChild(b);
  };
  bar('left:-1px;top:-1px;width:2px;height:2px;');
  const g = 'var(--fl-xh-gap,6px)';
  bar(`left:-1px;top:calc(${g} * -1 - 8px);width:2px;height:8px;`);
  bar(`left:-1px;top:${g};width:2px;height:8px;`);
  bar(`top:-1px;left:calc(${g} * -1 - 8px);width:8px;height:2px;`);
  bar(`top:-1px;left:${g};width:8px;height:2px;`);
  parent.appendChild(el);
  let timer: ReturnType<typeof setTimeout> | undefined;
  const off = bus.on('fire.local', () => {
    el.style.setProperty('--fl-xh-gap', '11px');
    if (timer !== undefined) clearTimeout(timer);
    timer = setTimeout(() => {
      el.style.setProperty('--fl-xh-gap', '6px');
      timer = undefined;
    }, CROSSHAIR_SPREAD_MS);
  });
  return {
    el,
    dispose: () => {
      off();
      if (timer !== undefined) clearTimeout(timer);
      el.remove();
    },
  };
}
