/** Action table, default bindings, conflict/reserved-key rules, schemes, import/export (UI_SPEC §5.2–5.5, M1-061a). */
export type Context = 'onFoot' | 'inVehicle' | 'gunner' | 'downed' | 'map' | 'menu' | 'chat';

export const ACTIONS = [
  'move_fwd', 'move_back', 'move_left', 'move_right', 'sprint', 'jump', 'crouch', 'prone', 'lean_l', 'lean_r', 'interact',
  'fire', 'ads', 'reload', 'fire_mode', 'throw', 'melee', 'map', 'scoreboard', 'squad_panel', 'pause',
] as const;
export type ActionId = (typeof ACTIONS)[number];

export interface Binding {
  primary?: string; // KeyboardEvent.code or Mouse0..Mouse4
  secondary?: string;
}
export type BindingMap = Record<ActionId, Binding>;

/** Contexts in which each action is live; conflicts only matter when two actions share a context. */
export const ACTION_CONTEXTS: Readonly<Record<ActionId, readonly Context[]>> = {
  move_fwd: ['onFoot', 'downed'], move_back: ['onFoot', 'downed'], move_left: ['onFoot', 'downed'], move_right: ['onFoot', 'downed'],
  sprint: ['onFoot'], jump: ['onFoot'], crouch: ['onFoot'], prone: ['onFoot'], lean_l: ['onFoot'], lean_r: ['onFoot'],
  interact: ['onFoot', 'inVehicle', 'downed'], fire: ['onFoot', 'inVehicle', 'gunner'], ads: ['onFoot', 'gunner'],
  reload: ['onFoot'], fire_mode: ['onFoot'], throw: ['onFoot'], melee: ['onFoot'],
  map: ['onFoot', 'inVehicle', 'gunner', 'downed', 'map'], scoreboard: ['onFoot', 'inVehicle', 'gunner', 'downed'],
  squad_panel: ['onFoot', 'inVehicle'], pause: ['onFoot', 'inVehicle', 'gunner', 'downed', 'map'],
};

export const DEFAULT_BINDINGS: Readonly<BindingMap> = {
  move_fwd: { primary: 'KeyW' }, move_back: { primary: 'KeyS' }, move_left: { primary: 'KeyA' }, move_right: { primary: 'KeyD' },
  sprint: { primary: 'ShiftLeft' }, jump: { primary: 'Space' }, crouch: { primary: 'KeyC' }, prone: { primary: 'KeyZ' },
  lean_l: { primary: 'KeyQ' }, lean_r: { primary: 'KeyE' }, interact: { primary: 'KeyF' },
  fire: { primary: 'Mouse0' }, ads: { primary: 'Mouse2' }, reload: { primary: 'KeyR' }, fire_mode: { primary: 'KeyX' },
  throw: { primary: 'KeyG' }, melee: { primary: 'KeyV' }, map: { primary: 'KeyM' }, scoreboard: { primary: 'Tab' },
  squad_panel: { primary: 'KeyU' }, pause: { primary: 'Escape' },
};

/** `pause` itself is fixed to Escape (the browser forces it anyway). */
export const FIXED_ACTIONS: ReadonlySet<ActionId> = new Set<ActionId>(['pause']);

export const RESERVED_CODES: ReadonlySet<string> = new Set(['Escape', 'F11', 'F12', 'MetaLeft', 'MetaRight', 'OSLeft', 'OSRight', 'PrintScreen', 'ContextMenu']);
export const RESERVED_KEY = 'ui.settings.controls.reserved';
export const CONFLICT_KEY = 'ui.settings.controls.conflict';

/** `code` may carry modifier prefixes ("Ctrl+KeyW"); modifier combos are refused (§5.3), Ctrl/Alt chords are browser-reserved. */
export function checkCode(code: string): { ok: true } | { ok: false; reasonKey: string } {
  if (/^(Ctrl|Control|Alt|Meta|Cmd)\+/i.test(code) || /\+/.test(code)) return { ok: false, reasonKey: RESERVED_KEY };
  if (RESERVED_CODES.has(code)) return { ok: false, reasonKey: RESERVED_KEY };
  if (!/^(Key[A-Z]|Digit\d|Numpad\w+|F\d{1,2}|Arrow\w+|Mouse[0-4]|Space|Enter|Tab|ShiftLeft|ShiftRight|CapsLock|Backquote|Minus|Equal|Bracket\w+|Semicolon|Quote|Comma|Period|Slash|Backslash|Backspace|Insert|Delete|Home|End|PageUp|PageDown)$/.test(code)) return { ok: false, reasonKey: RESERVED_KEY };
  return { ok: true };
}

const share = (a: ActionId, b: ActionId): boolean => ACTION_CONTEXTS[a].some((c) => ACTION_CONTEXTS[b].includes(c));

export function findConflict(map: Readonly<BindingMap>, action: ActionId, code: string): ActionId | undefined {
  return ACTIONS.find((o) => o !== action && share(o, action) && (map[o].primary === code || map[o].secondary === code));
}

export interface RebindResult {
  ok: boolean;
  conflict?: ActionId;
  reasonKey?: string;
}
export type ConflictChoice = 'swap' | 'unbind' | 'cancel';

export class Bindings {
  map: BindingMap = cloneMap(DEFAULT_BINDINGS);

  rebind(action: ActionId, slot: 'primary' | 'secondary', code: string): RebindResult {
    if (FIXED_ACTIONS.has(action)) return { ok: false, reasonKey: RESERVED_KEY };
    const c = checkCode(code);
    if (!c.ok) return { ok: false, reasonKey: c.reasonKey };
    const conflict = findConflict(this.map, action, code);
    if (conflict) return { ok: false, conflict, reasonKey: CONFLICT_KEY };
    this.map[action] = { ...this.map[action], [slot]: code };
    return { ok: true };
  }

  /** Resolves a conflict reported by `rebind`: swap exchanges the two keys; unbind clears the other action's slot. */
  resolve(action: ActionId, slot: 'primary' | 'secondary', code: string, choice: ConflictChoice): boolean {
    if (choice === 'cancel') return false;
    const other = findConflict(this.map, action, code);
    if (!other) return this.rebind(action, slot, code).ok;
    const oSlot = this.map[other].primary === code ? 'primary' : 'secondary';
    const mine = this.map[action][slot];
    this.map[action] = { ...this.map[action], [slot]: code };
    this.map[other] = { ...this.map[other], [oSlot]: choice === 'swap' ? mine : undefined };
    return true;
  }

  reset(): void {
    this.map = cloneMap(DEFAULT_BINDINGS);
  }

  /** Actions bound to a code in the given context. */
  actionsFor(code: string, ctx: Context): ActionId[] {
    return ACTIONS.filter((a) => ACTION_CONTEXTS[a].includes(ctx) && (this.map[a].primary === code || this.map[a].secondary === code));
  }

  export(): string {
    return JSON.stringify({ v: 1, bindings: this.map });
  }

  /** Rejects invalid JSON, unknown actions, reserved/invalid codes and in-context conflicts. Applies nothing on failure. */
  import(json: string): boolean {
    let data: unknown;
    try {
      data = JSON.parse(json);
    } catch {
      return false;
    }
    const b = (data as { v?: unknown; bindings?: Record<string, Binding> } | null)?.bindings;
    if (!b || (data as { v?: unknown }).v !== 1 || typeof b !== 'object') return false;
    const next = cloneMap(DEFAULT_BINDINGS);
    for (const a of ACTIONS) {
      const e = b[a];
      if (!e || typeof e !== 'object') return false;
      for (const slot of ['primary', 'secondary'] as const) {
        const code = e[slot];
        if (code === undefined) {
          next[a][slot] = undefined;
          continue;
        }
        if (typeof code !== 'string' || (!FIXED_ACTIONS.has(a) && !checkCode(code).ok && code !== 'Escape')) return false;
        next[a][slot] = code;
      }
    }
    for (const a of ACTIONS) for (const slot of ['primary', 'secondary'] as const) {
      const code = next[a][slot];
      if (code && findConflict(next, a, code)) return false;
    }
    this.map = next;
    return true;
  }
}

export function cloneMap(m: Readonly<BindingMap>): BindingMap {
  return Object.fromEntries(ACTIONS.map((a) => [a, { ...m[a] }])) as BindingMap;
}

/* ---- persistence with in-memory fallback ---- */
export interface StorageLike {
  getItem(k: string): string | null;
  setItem(k: string, v: string): void;
}
export class Persist {
  private mem = new Map<string, string>();
  /** True once a storage failure happened (UI shows the "not saved" hint once). */
  degraded = false;
  constructor(private readonly store: () => StorageLike | undefined) {}

  read(key: string): string | null {
    try {
      const v = this.store()?.getItem(key);
      if (v !== undefined && v !== null) return v;
    } catch {
      this.degraded = true;
    }
    return this.mem.get(key) ?? null;
  }
  write(key: string, value: string): void {
    this.mem.set(key, value);
    try {
      this.store()?.setItem(key, value);
    } catch {
      this.degraded = true;
    }
  }
}

/** Three scheme slots. */
export const SCHEME_SLOTS = 3;
export function saveScheme(p: Persist, slot: number, b: Bindings): boolean {
  if (slot < 0 || slot >= SCHEME_SLOTS) return false;
  p.write(`bindings.scheme.${slot}`, b.export());
  return true;
}
export function loadScheme(p: Persist, slot: number, b: Bindings): boolean {
  const s = p.read(`bindings.scheme.${slot}`);
  return s !== null && b.import(s);
}
