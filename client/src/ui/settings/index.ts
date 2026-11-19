import type { SettingsStore } from '../../game/settings';
import type { ActionId, Bindings, Context, Persist } from '../../input/bindings';
import { pauseNext, PAUSE_CLOSED, shouldPreventDefault, type PauseState } from '../../input/focus-guard';
import type { InputSampler } from '../../input/sampler';
import { setLocale, type Locale } from '../i18n';
import { button, loadActiveBindings } from './binding-editor';
import { makeCopy } from './copy';
import { MatchInputAdapter } from './input-adapter';
import { SettingsForm } from './settings-form';

export interface PauseSettingsOptions {
  root: HTMLElement;
  canvas: HTMLCanvasElement;
  /** Default-keymap sampler. Mount only during an active match; may coexist with M0's bubble-phase controller. */
  sampler: InputSampler;
  settings: SettingsStore;
  bindings: Bindings;
  /** Pass the same Persist instance to SettingsStore; prefix keys in its backing store if desired. */
  persist: Persist;
  getInputContext?: () => Context;
  /** Unmapped sampler actions (e.g. map/interact) are routed here as press/release pairs. */
  onAction?: (action: ActionId, pressed: boolean) => void;
  isMapOpen?: () => boolean;
  /** Close the map synchronously (Escape or an unexpected lock loss). The adapter shows a relock control. */
  onCloseMap?: () => void;
  /** Caller disconnects and routes to the server browser; invoked once after confirmation and teardown. */
  onLeave: () => void;
  onRedeploy?: () => void;
  onTeamChange?: () => void;
  teamCooldownSeconds?: () => number;
  onCopyDiagnostics?: () => void | Promise<void>;
}

export interface PauseSettingsHandle {
  open(): void;
  /** Call synchronously in a user gesture. No automatic relock is attempted. */
  resume(): void;
  /** After setting the map-open state, call this instead of exitPointerLock(). Safe in a map press callback. */
  suspendForMap(): void;
  /** Clear held input at an external game-context transition even if no DOM event occurs. */
  releaseInput(): void;
  dispose(): void;
}

interface LockRequest { promise?: Promise<void> }

const latestMount = new WeakMap<HTMLCanvasElement, object>();

export function mountPauseSettings(options: PauseSettingsOptions): PauseSettingsHandle {
  const { root, canvas, sampler, bindings, persist, settings } = options;
  const doc = canvas.ownerDocument;
  const win = doc.defaultView!;
  const owner = {};
  latestMount.set(canvas, owner);
  loadActiveBindings(persist, bindings);
  const mapIsOpen = (): boolean => options.isMapOpen?.() === true;
  const input = new MatchInputAdapter(sampler, bindings, settings, () => mapIsOpen() ? 'map' : options.getInputContext?.() ?? 'onFoot', options.onAction);
  input.releaseAll();

  const panel = doc.createElement('section');
  panel.dataset.pauseSettings = '';
  panel.setAttribute('role', 'dialog');
  const warning = doc.createElement('p');
  warning.setAttribute('role', 'status');
  warning.hidden = true;
  const content = doc.createElement('div');
  panel.append(warning, content);
  root.append(panel);

  let disposed = false;
  let state = PAUSE_CLOSED as PauseState;
  let page: 'pause' | 'settings' | 'redeploy' = 'pause';
  let group: ConstructorParameters<typeof SettingsForm>[0]['group'] = 'audio';
  let form: SettingsForm | undefined;
  let locked = doc.pointerLockElement === canvas;
  let focused = doc.hasFocus();
  let pending: LockRequest | undefined;
  let expectedMapUnlock = false;
  let teamButton: HTMLButtonElement | undefined;
  let capturedMouse: EventTarget | null = null;
  const cleanups: (() => void)[] = [];

  function locale(): Locale { return settings.get('ui.lang') === 'zh-Hans' ? 'zh-Hans' : 'en'; }
  function updateWarning(): void {
    warning.hidden = !persist.degraded;
    warning.textContent = persist.degraded ? makeCopy(locale())('storage') : '';
  }
  function updateTeamButton(): void {
    if (!teamButton) return;
    const seconds = Math.max(0, Math.ceil(options.teamCooldownSeconds?.() ?? 0));
    teamButton.disabled = !options.onTeamChange || seconds > 0;
    const clock = `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
    teamButton.textContent = `${makeCopy(locale())('team')}${seconds > 0 ? ` (${clock})` : ''}`;
  }
  function showPause(): void { page = 'pause'; render(true); }
  function render(focusPanel = false): void {
    const copy = makeCopy(locale());
    panel.hidden = !state.open && !state.needClick && pending === undefined;
    panel.setAttribute('aria-label', copy(page === 'settings' ? 'menu.settings' : 'pause'));
    panel.dataset.view = state.open ? page : state.needClick ? 'click-to-continue' : pending !== undefined ? 'locking' : 'closed';
    content.replaceChildren();
    form = undefined;
    teamButton = undefined;
    updateWarning();
    if (panel.hidden) return;
    if (pending !== undefined && !state.open) {
      const status = doc.createElement('p');
      status.setAttribute('role', 'status');
      status.textContent = copy('locked');
      content.append(status);
    } else if (state.needClick) {
      content.append(button(doc, copy('ui.pause.clickToContinue'), resume));
    } else if (page === 'settings') {
      form = new SettingsForm({ doc, settings, bindings, persist, copy, group,
        onGroup: (next) => { group = next; }, onBindingsChange: () => { input.releaseAll(); updateWarning(); },
        onBack: showPause, onCopyDiagnostics: options.onCopyDiagnostics });
      content.append(form.root);
    } else {
      const heading = doc.createElement('h2');
      heading.textContent = copy('pause');
      const status = doc.createElement('p');
      status.textContent = copy('ui.pause.inProgress');
      content.append(heading, status);
      if (state.confirmLeave || page === 'redeploy') {
        const leaving = state.confirmLeave;
        const prompt = doc.createElement('p');
        prompt.textContent = copy(leaving ? 'ui.pause.leaveConfirm' : 'redeployConfirm');
        content.append(prompt,
          button(doc, copy('cancel'), () => { state = pauseNext(state, 'cancelLeave'); showPause(); }),
          button(doc, copy('confirm'), () => {
            if (leaving) { dispose(); options.onLeave(); }
            else { showPause(); options.onRedeploy?.(); }
          }));
      } else {
        teamButton = button(doc, copy('team'), () => {
          updateTeamButton();
          if (!teamButton?.disabled) options.onTeamChange?.();
        });
        updateTeamButton();
        content.append(button(doc, copy('ui.pause.continue'), resume),
          button(doc, copy('redeploy'), () => { page = 'redeploy'; render(true); }, !options.onRedeploy), teamButton,
          button(doc, copy('menu.settings'), () => { page = 'settings'; render(true); }),
          button(doc, copy('report'), () => {}, true), button(doc, copy('vote'), () => {}, true), button(doc, copy('squad'), () => {}, true),
          button(doc, copy('ui.pause.leave'), () => { state = pauseNext(state, 'leave'); render(true); }));
      }
    }
    if (focusPanel) content.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus({ preventScroll: true });
  }

  function refuse(request: LockRequest): void {
    if (disposed || pending !== request) return;
    pending = undefined;
    expectedMapUnlock = false;
    input.releaseAll();
    if (!state.open && !mapIsOpen()) state = pauseNext(state, 'lockRefused');
    render(true);
  }
  function resume(): void {
    if (disposed || pending !== undefined || mapIsOpen()) return;
    if (doc.pointerLockElement === canvas) {
      // An explicit open/blur has requested an unlock but its event hasn't arrived yet.
      if (state.open || state.needClick) return;
      locked = true;
      return;
    }
    focused = true; // Called in a fresh user gesture, including cross-frame pointer activation.
    const request: LockRequest = {};
    pending = request;
    page = 'pause';
    state = PAUSE_CLOSED;
    let result: ReturnType<HTMLCanvasElement['requestPointerLock']> | undefined;
    try {
      if (typeof canvas.requestPointerLock !== 'function') { refuse(request); return; }
      // Must run in this stack, not after rendering, an awaited fallback or a timer.
      result = canvas.requestPointerLock();
    } catch { refuse(request); return; }
    if (result) {
      request.promise = result;
      void result.then(() => {
        if (disposed) releaseLateLock();
        else if (pending === request) lockChanged();
      }, () => refuse(request));
    }
    if (pending === request) render();
  }
  function open(): void {
    if (disposed) return;
    input.releaseAll();
    form?.editor.cancel();
    const alreadyOpen = state.open;
    state = pauseNext({ ...state, needClick: false }, 'blur');
    if (doc.pointerLockElement === canvas) doc.exitPointerLock?.();
    if (!alreadyOpen) showPause();
  }
  function suspendForMap(): void {
    if (disposed) return;
    const lockIsActive = doc.pointerLockElement === canvas;
    expectedMapUnlock = lockIsActive || locked || pending !== undefined;
    state = PAUSE_CLOSED;
    page = 'pause';
    input.releaseAll();
    if (lockIsActive) doc.exitPointerLock?.();
    render();
  }
  function recoverAfterMap(): void {
    input.releaseAll();
    if (!disposed && !state.open && pending === undefined && doc.pointerLockElement !== canvas) {
      state = pauseNext(state, 'lockRefused');
      render(true);
    }
  }
  function closeMap(): void {
    options.onCloseMap?.();
    if (!mapIsOpen()) recoverAfterMap();
  }
  function lockChanged(): void {
    const wasLocked = locked;
    const wasPending = pending !== undefined;
    locked = doc.pointerLockElement === canvas;
    if (locked) {
      pending = undefined;
      // Ignore a late grant after an explicit open, tab blur, or a map suspension.
      if (state.open || expectedMapUnlock || mapIsOpen() || !focused || doc.hidden) { doc.exitPointerLock?.(); return; }
      if (!wasLocked || wasPending) {
        state = pauseNext(state, 'lockGranted');
        showPause();
      }
    } else if (wasLocked || expectedMapUnlock || wasPending) {
      pending = undefined;
      input.releaseAll();
      if (expectedMapUnlock) {
        expectedMapUnlock = false;
        if (!mapIsOpen()) recoverAfterMap();
        return;
      }
      if (state.open) return;
      const mapOpen = mapIsOpen();
      state = pauseNext(state, 'lockLost', mapOpen);
      if (mapOpen) closeMap();
      else showPause();
    }
  }
  function routeUnlockedMap(code: string, repeat = false): boolean {
    if (!mapIsOpen() || state.open || !focused || doc.hidden || !bindings.actionsFor(code, 'map').includes('map')) return false;
    if (!repeat) {
      if (options.onAction) {
        input.press(code);
        // The caller may optionally have requested a relock within this same gesture already.
        if (!mapIsOpen()) recoverAfterMap();
      } else closeMap();
    }
    return true;
  }
  function listen<T extends Event>(target: EventTarget, type: string, fn: (event: T) => void, capture = false): void {
    const listener = fn as EventListener;
    target.addEventListener(type, listener, capture);
    cleanups.push(() => target.removeEventListener(type, listener, capture));
  }
  function eat(e: Event): void { e.preventDefault(); e.stopImmediatePropagation(); }
  const ownsLock = (): boolean => doc.pointerLockElement === canvas;
  const routingInput = (): boolean => ownsLock() && !expectedMapUnlock && !state.open && focused && !doc.hidden;

  listen(doc, 'pointerlockchange', lockChanged);
  listen(doc, 'pointerlockerror', () => { if (pending && !pending.promise) refuse(pending); });
  listen(doc, 'visibilitychange', () => { if (doc.hidden) open(); });
  listen(win, 'blur', () => { focused = false; open(); });
  listen(win, 'focus', () => { focused = true; });
  listen<BeforeUnloadEvent>(win, 'beforeunload', (e) => { e.preventDefault(); e.returnValue = true; });
  listen<MouseEvent>(win, 'contextmenu', (e) => { if (e.target === canvas || capturedMouse || ownsLock()) e.preventDefault(); }, true);
  listen<MouseEvent>(win, 'auxclick', (e) => { if (capturedMouse || (e.button === 1 && (e.target === canvas || ownsLock()))) e.preventDefault(); }, true);
  listen<MouseEvent>(win, 'click', (e) => {
    if (capturedMouse && e.target === capturedMouse) { capturedMouse = null; eat(e); return; }
    if (e.target !== canvas) return;
    eat(e); // Own the gesture instead of M0's target listener.
    if (!state.open && !mapIsOpen()) resume();
  }, true);
  listen<KeyboardEvent>(win, 'keydown', (e) => {
    if (form?.editor.handleKey(e)) { eat(e); return; }
    if (e.code === 'F5' || (e.ctrlKey && e.code === 'KeyR')) e.preventDefault();
    if (e.code === 'Escape') {
      if (ownsLock()) { e.stopImmediatePropagation(); return; } // Never prevent browser Esc unlock.
      if (e.repeat) return;
      if (state.open && (page === 'settings' || page === 'redeploy' || state.confirmLeave)) {
        state = pauseNext(state, 'cancelLeave'); showPause(); eat(e);
      } else if (state.open || state.needClick) {
        resume(); eat(e);
      } else if (mapIsOpen()) {
        closeMap(); eat(e);
      }
      return;
    }
    if (!ownsLock()) {
      if (!(e.ctrlKey || e.altKey || e.metaKey) && routeUnlockedMap(e.code, e.repeat)) eat(e);
      else if (locked) e.stopImmediatePropagation(); // M0 may not have seen the unlock event yet.
      return;
    }
    if (!routingInput()) { e.stopImmediatePropagation(); return; }
    // Keep reserved browser shortcuts out of gameplay, but don't swallow the shortcut itself.
    if (e.ctrlKey || e.altKey || e.metaKey) { e.stopImmediatePropagation(); return; }
    if (!e.repeat && input.press(e.code)) e.preventDefault();
    if (shouldPreventDefault(e.code, true)) e.preventDefault();
    e.stopImmediatePropagation();
  }, true);
  listen<KeyboardEvent>(win, 'keyup', (e) => {
    if (!ownsLock()) {
      if (mapIsOpen()) input.release(e.code);
      if (locked) e.stopImmediatePropagation();
      return;
    }
    if (routingInput()) input.release(e.code);
    e.stopImmediatePropagation();
  }, true);
  listen<MouseEvent>(win, 'mousedown', (e) => {
    capturedMouse = null;
    if (form?.editor.handleMouse(e)) { capturedMouse = e.target; eat(e); return; }
    if (!ownsLock()) {
      if (routeUnlockedMap(`Mouse${e.button}`)) { capturedMouse = e.target; eat(e); }
      else if (locked) e.stopImmediatePropagation();
      return;
    }
    if (!routingInput()) { e.stopImmediatePropagation(); return; }
    if (input.press(`Mouse${e.button}`) || e.button === 1) e.preventDefault();
    e.stopImmediatePropagation();
  }, true);
  listen<MouseEvent>(win, 'mouseup', (e) => {
    if (!ownsLock()) {
      if (mapIsOpen()) input.release(`Mouse${e.button}`);
      if (locked) e.stopImmediatePropagation();
      return;
    }
    if (routingInput()) input.release(`Mouse${e.button}`);
    e.stopImmediatePropagation();
  }, true);
  listen<MouseEvent>(win, 'mousemove', (e) => {
    if (!ownsLock() && !locked) return;
    if (routingInput()) input.mouseMove(e.movementX, e.movementY);
    e.stopImmediatePropagation();
  }, true);

  setLocale(locale());
  cleanups.push(settings.onChange(({ key }) => {
    if (key === 'ui.lang') { setLocale(locale()); render(true); }
    else { form?.update(settings, key); updateWarning(); }
  }));
  if (options.teamCooldownSeconds) {
    const timer = win.setInterval(updateTeamButton, 1000);
    cleanups.push(() => win.clearInterval(timer));
  }
  render();

  function releaseLateLock(): void {
    if (disposed && latestMount.get(canvas) === owner && doc.pointerLockElement === canvas) doc.exitPointerLock?.();
  }
  function dispose(): void {
    if (disposed) return;
    disposed = true;
    if (pending && !pending.promise && doc.pointerLockElement !== canvas) {
      // A legacy void-returning request may finish after teardown; observe at most its next transition.
      doc.addEventListener('pointerlockchange', releaseLateLock, { once: true, signal: AbortSignal.timeout(1000) });
    }
    pending = undefined;
    for (const cleanup of cleanups.splice(0)) cleanup();
    input.releaseAll();
    if (doc.pointerLockElement === canvas) doc.exitPointerLock?.();
    panel.remove();
    content.replaceChildren();
    form = undefined;
  }
  return { open, resume, suspendForMap, releaseInput: () => input.releaseAll(), dispose };
}
