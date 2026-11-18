/** Focus / pause rules (UI_SPEC §5.5), pure so they can be tested without a DOM. */
export interface PauseState {
  open: boolean;
  confirmLeave: boolean;
  /** Pointer-lock re-request was refused → show the "click to continue" overlay (z-index 80). */
  needClick: boolean;
}
export const PAUSE_CLOSED: PauseState = { open: false, confirmLeave: false, needClick: false };

export type PauseEvent = 'lockLost' | 'continue' | 'lockRefused' | 'lockGranted' | 'leave' | 'confirmLeave' | 'cancelLeave' | 'blur' | 'mapOpen' | 'mapClose';

export function pauseNext(s: PauseState, e: PauseEvent, mapOpen = false): PauseState {
  switch (e) {
    case 'lockLost':
    case 'blur':
      // Esc closes the map first; only a second Esc opens the menu (UI_SPEC §4.7.7).
      return mapOpen && e === 'lockLost' ? s : { ...s, open: true };
    case 'continue':
      return s.open ? { ...s, open: false, confirmLeave: false } : s;
    case 'lockRefused':
      return { open: false, confirmLeave: false, needClick: true };
    case 'lockGranted':
      return PAUSE_CLOSED;
    case 'leave':
      return s.open ? { ...s, confirmLeave: true } : s;
    case 'cancelLeave':
      return { ...s, confirmLeave: false };
    case 'confirmLeave':
      return PAUSE_CLOSED;
    default:
      return s;
  }
}

/** Clears all held keys/buttons (called on `blur`/`visibilitychange`). */
export function clearHeld(held: Map<string, boolean> | Set<string>): void {
  held.clear();
}

/** Keys the page must swallow in-game: F5, Tab, context menu, middle-click autoscroll. */
export function shouldPreventDefault(code: string, inGame: boolean): boolean {
  return inGame && (code === 'F5' || code === 'Tab' || code === 'ContextMenu' || code === 'Mouse1');
}
