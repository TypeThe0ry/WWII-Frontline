import { SETTINGS, type SettingsStore } from '../../game/settings';
import type { Bindings, Persist } from '../../input/bindings';
import { BindingEditor, button } from './binding-editor';
import type { CopyKey } from './copy';

const GROUPS = {
  audio: ['audio.master', 'audio.sfx', 'audio.ui'],
  controls: ['input.sensH', 'input.sensV', 'input.adsMult', 'input.invertY'],
  video: ['video.fov', 'quality.tier'],
  language: ['ui.lang'],
  diagnostics: ['net.showStats'],
} as const satisfies Record<string, readonly CopyKey[]>;

type Group = keyof typeof GROUPS;
interface FormOptions {
  doc: Document;
  settings: SettingsStore;
  bindings: Bindings;
  persist: Persist;
  copy(key: CopyKey): string;
  group: Group;
  onGroup(group: Group): void;
  onBindingsChange(): void;
  onCopyDiagnostics?: () => void | Promise<void>;
  onBack(): void;
}

export class SettingsForm {
  readonly root: HTMLElement;
  readonly editor: BindingEditor;
  private readonly controls = new Map<string, HTMLInputElement | HTMLSelectElement>();

  constructor({ doc, settings, bindings, persist, copy, group, onGroup, onBindingsChange, onCopyDiagnostics, onBack }: FormOptions) {
    this.root = doc.createElement('section');
    const heading = doc.createElement('h2');
    heading.textContent = copy('menu.settings');
    this.root.append(heading, button(doc, copy('back'), onBack));
    const nav = doc.createElement('nav');
    nav.setAttribute('aria-label', copy('menu.settings'));
    const groups = Object.keys(GROUPS) as Group[];
    const panels = new Map<Group, HTMLElement>();
    const navigation = new Map<Group, HTMLButtonElement>();
    const selectGroup = (next: Group): void => {
      for (const g of groups) {
        panels.get(g)!.hidden = g !== next;
        navigation.get(g)!.setAttribute('aria-pressed', String(g === next));
      }
      this.editor.cancel();
      onGroup(next);
    };
    this.editor = new BindingEditor(doc, bindings, persist, copy, onBindingsChange);
    this.root.append(nav);
    for (const g of groups) {
      const tab = button(doc, copy(g), () => selectGroup(g));
      navigation.set(g, tab);
      nav.append(tab);
      const panel = doc.createElement('fieldset');
      panels.set(g, panel);
      const legend = doc.createElement('legend');
      legend.textContent = copy(g);
      panel.append(legend);
      for (const key of GROUPS[g]) {
        const spec = SETTINGS[key]!;
        const label = doc.createElement('label');
        label.textContent = `${copy(key)} `;
        if (spec.kind === 'enum') {
          const input = doc.createElement('select');
          for (const value of spec.options!) {
            const item = doc.createElement('option');
            item.value = value;
            item.textContent = value === 'en' ? 'English' : value === 'zh-Hans' ? '简体中文' : copy(value as CopyKey);
            input.append(item);
          }
          input.addEventListener('change', () => settings.set(key, input.value));
          this.controls.set(key, input);
          label.append(input);
        } else {
          const input = doc.createElement('input');
          input.type = spec.kind === 'toggle' ? 'checkbox' : 'range';
          if (spec.kind === 'range') {
            input.min = String(spec.min);
            input.max = String(spec.max);
            input.step = String(spec.step);
          }
          input.addEventListener('input', () => settings.set(key, spec.kind === 'toggle' ? input.checked : input.valueAsNumber));
          this.controls.set(key, input);
          label.append(input);
        }
        const row = doc.createElement('div');
        row.append(label);
        panel.append(row);
        this.update(settings, key);
      }
      if (g === 'controls') panel.append(this.editor.root);
      if (g === 'diagnostics') {
        const feedback = doc.createElement('p');
        feedback.setAttribute('role', 'status');
        panel.append(button(doc, copy('copy'), async () => {
          try { await onCopyDiagnostics?.(); feedback.textContent = copy('copied'); }
          catch { feedback.textContent = copy('copyFailed'); }
        }, !onCopyDiagnostics), feedback);
      }
      this.root.append(panel);
    }
    selectGroup(group);
  }

  update(settings: SettingsStore, key: string): void {
    const control = this.controls.get(key);
    if (!control) return;
    const value = settings.get(key);
    if (control instanceof HTMLInputElement && control.type === 'checkbox') control.checked = value === true;
    else control.value = String(value);
  }
}
