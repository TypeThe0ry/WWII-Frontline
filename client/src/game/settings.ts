/** Settings store (M1-061a): clamped values, persistence with fallback, `settings.changed` notifications. */
import { Persist } from '../input/bindings';

export interface SettingSpec {
  kind: 'range' | 'toggle' | 'enum';
  min?: number;
  max?: number;
  step?: number;
  options?: readonly string[];
  def: number | boolean | string;
}

export const SETTINGS: Readonly<Record<string, SettingSpec>> = {
  'audio.master': { kind: 'range', min: 0, max: 1, step: 0.01, def: 0.8 },
  'audio.sfx': { kind: 'range', min: 0, max: 1, step: 0.01, def: 1 },
  'audio.ui': { kind: 'range', min: 0, max: 1, step: 0.01, def: 0.7 },
  'input.sensH': { kind: 'range', min: 0.1, max: 5, step: 0.01, def: 1 },
  'input.sensV': { kind: 'range', min: 0.1, max: 5, step: 0.01, def: 1 },
  'input.adsMult': { kind: 'range', min: 0.2, max: 1.5, step: 0.01, def: 0.8 },
  'input.invertY': { kind: 'toggle', def: false },
  'video.fov': { kind: 'range', min: 60, max: 100, step: 1, def: 80 },
  'quality.tier': { kind: 'enum', options: ['low', 'medium', 'high'], def: 'medium' },
  'ui.lang': { kind: 'enum', options: ['en', 'zh-Hans'], def: 'en' },
  'net.showStats': { kind: 'toggle', def: false },
};

export type SettingValue = number | boolean | string;
export function clampSetting(key: string, v: SettingValue): SettingValue {
  const s = SETTINGS[key];
  if (!s) return v;
  if (s.kind === 'range') {
    const n = typeof v === 'number' && Number.isFinite(v) ? v : (s.def as number);
    const c = Math.min(s.max!, Math.max(s.min!, n));
    return s.step ? Math.round(c / s.step) * s.step : c;
  }
  if (s.kind === 'toggle') return typeof v === 'boolean' ? v : (s.def as boolean);
  return s.options!.includes(v as string) ? v : s.def;
}

export type ChangeListener = (e: { key: string; value: SettingValue }) => void;

export class SettingsStore {
  private values = new Map<string, SettingValue>();
  private listeners: ChangeListener[] = [];
  constructor(private readonly persist: Persist) {
    for (const k of Object.keys(SETTINGS)) this.values.set(k, SETTINGS[k]!.def);
    const raw = persist.read('settings');
    if (raw) {
      try {
        const o = JSON.parse(raw) as Record<string, SettingValue>;
        for (const k of Object.keys(SETTINGS)) if (k in o) this.values.set(k, clampSetting(k, o[k]!));
      } catch {
        /* corrupt → defaults */
      }
    }
  }
  get<T extends SettingValue>(key: string): T {
    return this.values.get(key) as T;
  }
  onChange(l: ChangeListener): () => void {
    this.listeners.push(l);
    return () => void (this.listeners = this.listeners.filter((x) => x !== l));
  }
  set(key: string, value: SettingValue): boolean {
    if (!(key in SETTINGS)) return false;
    const v = clampSetting(key, value);
    if (this.values.get(key) === v) return true;
    this.values.set(key, v);
    this.persist.write('settings', JSON.stringify(Object.fromEntries(this.values)));
    for (const l of this.listeners) l({ key, value: v });
    return true;
  }
}
