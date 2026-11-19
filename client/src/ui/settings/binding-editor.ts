import { ACTIONS, Bindings, FIXED_ACTIONS, SCHEME_SLOTS, loadScheme, saveScheme,
  type ActionId, type Binding, type ConflictChoice, type Persist } from '../../input/bindings';
import type { CopyKey } from './copy';

type Slot = keyof Binding;
type Copy = (key: CopyKey) => string;
type Capture = { action: ActionId; slot: Slot; button: HTMLButtonElement };
const ACTIVE_SCHEME = 'bindings.active';

/** Check the fixed Escape binding at the DOM boundary, including on load of user-edited JSON. */
function validPause(bindings: Bindings): boolean {
  return bindings.map.pause.primary === 'Escape' && bindings.map.pause.secondary === undefined
    && ACTIONS.every((a) => a === 'pause' || (bindings.map[a].primary !== 'Escape' && bindings.map[a].secondary !== 'Escape'));
}

export function loadActiveBindings(persist: Persist, bindings: Bindings): void {
  const raw = persist.read(ACTIVE_SCHEME);
  const candidate = new Bindings();
  if (raw && candidate.import(raw) && validPause(candidate)) bindings.map = candidate.map;
}

export function button(doc: Document, label: string, run: () => void, disabled = false): HTMLButtonElement {
  const node = doc.createElement('button');
  node.type = 'button';
  node.textContent = label;
  node.disabled = disabled;
  node.addEventListener('click', run);
  return node;
}

export class BindingEditor {
  readonly root: HTMLElement;
  private readonly table: HTMLTableElement;
  private readonly feedback: HTMLDivElement;
  private capture?: Capture;
  private cancelButton?: HTMLButtonElement;
  private conflictTrigger?: HTMLButtonElement;

  constructor(
    private readonly doc: Document,
    private readonly bindings: Bindings,
    private readonly persist: Persist,
    private readonly copy: Copy,
    private readonly changed: () => void,
  ) {
    this.root = doc.createElement('fieldset');
    const legend = doc.createElement('legend');
    legend.textContent = copy('bindings');
    this.table = doc.createElement('table');
    this.feedback = doc.createElement('div');
    this.feedback.setAttribute('role', 'status');
    this.drawTable();
    this.root.append(legend, this.feedback, this.table);

    const scheme = doc.createElement('select');
    scheme.setAttribute('aria-label', copy('scheme'));
    for (let i = 0; i < SCHEME_SLOTS; i++) {
      const option = doc.createElement('option');
      option.value = String(i);
      option.textContent = `${copy('scheme')} ${i + 1}`;
      scheme.append(option);
    }
    const json = doc.createElement('textarea');
    json.setAttribute('aria-label', copy('json'));
    this.root.append(scheme,
      button(doc, copy('save'), () => {
        saveScheme(persist, Number(scheme.value), bindings);
        this.show('saved');
        changed();
      }),
      button(doc, copy('load'), () => {
        const candidate = new Bindings();
        if (!loadScheme(persist, Number(scheme.value), candidate) || !validPause(candidate)) { this.show('invalid'); return; }
        bindings.map = candidate.map;
        this.saveActive('loaded');
      }),
      json,
      button(doc, copy('export'), () => { json.value = bindings.export(); json.focus(); json.select(); }),
      button(doc, copy('import'), () => {
        const candidate = new Bindings();
        if (!candidate.import(json.value) || !validPause(candidate)) { this.show('invalid'); return; }
        bindings.map = candidate.map;
        this.saveActive('loaded');
      }));
  }

  handleKey(e: KeyboardEvent): boolean {
    if (!this.capture) {
      if (e.code !== 'Escape' || !this.conflictTrigger) return false;
      this.cancel();
      return true;
    }
    if (e.code === 'Escape') this.cancel();
    else if (!e.repeat) {
      const modifier = e.ctrlKey ? 'Ctrl+' : e.altKey ? 'Alt+' : e.metaKey ? 'Meta+' : '';
      this.submit(`${modifier}${e.code}`);
    }
    return true;
  }

  handleMouse(e: MouseEvent): boolean {
    if (!this.capture || (e.target instanceof Node && this.cancelButton?.contains(e.target))) return false;
    this.submit(`Mouse${e.button}`);
    return true;
  }

  cancel(): void {
    const trigger = this.capture?.button ?? this.conflictTrigger;
    this.show();
    trigger?.focus();
  }

  private drawTable(): void {
    this.table.replaceChildren();
    const header = this.table.createTHead().insertRow();
    for (const text of ['', this.copy('primary'), this.copy('secondary')]) {
      const th = this.doc.createElement('th');
      th.scope = 'col';
      th.textContent = text;
      header.append(th);
    }
    const body = this.table.createTBody();
    for (const action of ACTIONS) {
      const row = body.insertRow();
      const label = this.doc.createElement('th');
      label.scope = 'row';
      label.textContent = action;
      row.append(label);
      for (const slot of ['primary', 'secondary'] as const) {
        const code = this.bindings.map[action][slot] ?? this.copy('unassigned');
        const control = button(this.doc, code, () => {
          this.show('press');
          this.capture = { action, slot, button: control };
          this.cancelButton = button(this.doc, this.copy('cancel'), () => this.cancel());
          this.feedback.append(' ', this.cancelButton);
        }, FIXED_ACTIONS.has(action));
        control.setAttribute('aria-label', `${action} ${this.copy(slot)}: ${code}`);
        control.dataset.binding = `${action}.${slot}`;
        row.insertCell().append(control);
      }
    }
  }

  private submit(code: string): void {
    const { action, slot, button: trigger } = this.capture!;
    const result = this.bindings.rebind(action, slot, code);
    if (result.ok) { this.saveActive(undefined, action, slot); return; }
    if (!result.conflict) { this.show('ui.settings.controls.reserved'); trigger.focus(); return; }
    this.show('ui.settings.controls.conflict');
    this.conflictTrigger = trigger;
    trigger.setAttribute('aria-invalid', 'true');
    this.feedback.append(`: ${result.conflict}. `);
    const resolve = (choice: ConflictChoice): void => {
      if (choice === 'cancel') { this.show(); trigger.focus(); return; }
      const candidate = new Bindings();
      candidate.import(this.bindings.export());
      // A swap can create a second conflict in a different shared context. Never apply a partial edit.
      if (!candidate.resolve(action, slot, code, choice) || !candidate.import(candidate.export())) {
        this.feedback.append(` ${this.copy('invalid')}`);
        return;
      }
      this.bindings.map = candidate.map;
      this.saveActive(undefined, action, slot);
    };
    const swap = button(this.doc, this.copy('swap'), () => resolve('swap'));
    this.feedback.append(swap, button(this.doc, this.copy('unbind'), () => resolve('unbind')),
      button(this.doc, this.copy('cancel'), () => resolve('cancel')));
    swap.focus();
  }

  private show(key?: CopyKey): void {
    this.capture = undefined;
    this.cancelButton = undefined;
    this.conflictTrigger?.removeAttribute('aria-invalid');
    this.conflictTrigger = undefined;
    this.feedback.textContent = key === 'ui.settings.controls.conflict' ? `⚠ ${this.copy(key)}` : key ? this.copy(key) : '';
  }

  private saveActive(key?: CopyKey, action?: ActionId, slot?: Slot): void {
    this.persist.write(ACTIVE_SCHEME, this.bindings.export());
    this.show(key);
    this.drawTable();
    this.changed();
    if (action) this.table.querySelector<HTMLElement>(`[data-binding="${action}.${slot}"]`)?.focus();
  }
}
