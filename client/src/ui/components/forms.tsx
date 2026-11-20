import type { JSX } from 'preact';
import { useId } from 'preact/hooks';
import { t } from '../i18n';

interface FieldProps<T> {
  id?: string;
  name?: string;
  labelKey: string;
  hintKey?: string;
  error?: string;
  disabled?: boolean;
  required?: boolean;
  value: T;
  onChange(value: T): void;
}

type FieldAttrs = Pick<FieldProps<unknown>, 'name' | 'disabled' | 'required'> & {
  id: string;
  'aria-describedby'?: string;
  'aria-invalid'?: boolean;
};

function Field({ id, name, labelKey, hintKey, error, disabled, required, control }:
  Omit<FieldProps<unknown>, 'value' | 'onChange'> & { control(attrs: FieldAttrs): JSX.Element }) {
  const auto = useId();
  const input = id ?? auto;
  const hintId = `${input}-hint`;
  const errorId = `${input}-error`;
  return <div>
    <label for={input}>{t(labelKey)}</label>
    {control({ id: input, name, disabled, required, 'aria-invalid': error ? true : undefined,
      'aria-describedby': [hintKey && hintId, error && errorId].filter(Boolean).join(' ') || undefined })}
    {hintKey && <span id={hintId}>{t(hintKey)}</span>}
    {error && <span id={errorId}>{error}</span>}
  </div>;
}

export interface SelectOption { value: string; labelKey: string; disabled?: boolean }
export function Select({ options, value, onChange, ...field }: FieldProps<string> & { options: readonly SelectOption[] }) {
  return <Field {...field} control={attrs => <select {...attrs} value={value} onChange={e => onChange(e.currentTarget.value)}>
    {options.map(o => <option key={o.value} value={o.value} disabled={o.disabled}>{t(o.labelKey)}</option>)}
  </select>} />;
}

export function Slider({ min, max, step, value, onChange, ...field }: FieldProps<number> & { min?: number; max?: number; step?: number | 'any' }) {
  return <Field {...field} control={attrs => <input {...attrs} type="range" min={min} max={max} step={step} value={value}
    onInput={e => onChange(e.currentTarget.valueAsNumber)} />} />;
}

export function Toggle({ value, onChange, ...field }: FieldProps<boolean>) {
  return <Field {...field} control={attrs => <input {...attrs} type="checkbox" checked={value}
    onChange={e => onChange(e.currentTarget.checked)} />} />;
}

export function TextField({ value, onChange, placeholder, ...field }: FieldProps<string> & { placeholder?: string }) {
  return <Field {...field} control={attrs => <input {...attrs} type="text" value={value} placeholder={placeholder}
    onInput={e => onChange(e.currentTarget.value)} />} />;
}
