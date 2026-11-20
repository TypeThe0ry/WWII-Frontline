import type { ComponentChildren } from 'preact';
import { useId } from 'preact/hooks';
import { t } from '../i18n';

export interface TabItem { id: string; labelKey: string; disabled?: boolean; panel?: ComponentChildren }
interface ChoiceProps { label: string; items: readonly TabItem[]; value: string; onChange(id: string): void }

export function nextTab(items: readonly TabItem[], id: string, key: string): string | undefined {
  const enabled = items.filter(item => !item.disabled);
  const i = enabled.findIndex(item => item.id === id);
  switch (key) {
    case 'Home': return enabled[0]?.id;
    case 'End': return enabled.at(-1)?.id;
    case 'ArrowLeft': return enabled[(i - 1 + enabled.length) % enabled.length]?.id;
    case 'ArrowRight': return enabled[(i + 1) % enabled.length]?.id;
  }
}

export function Tabs({ label, items, value, onChange }: ChoiceProps) {
  const base = useId();
  const active = items.find(i => i.id === value && !i.disabled)?.id ?? items.find(i => !i.disabled)?.id;
  const tabId = (id: string) => `${base}-tab-${encodeURIComponent(id)}`;
  const panelId = (id: string) => `${base}-panel-${encodeURIComponent(id)}`;
  return <>
    <div role="tablist" aria-label={label}>
      {items.map(item => <button type="button" role="tab" key={item.id} id={tabId(item.id)} disabled={item.disabled}
        aria-selected={item.id === active} aria-controls={panelId(item.id)} tabIndex={item.id === active ? 0 : -1}
        onClick={() => onChange(item.id)} onKeyDown={e => {
          const next = nextTab(items, item.id, e.key);
          if (next === undefined) return;
          e.preventDefault();
          e.currentTarget.ownerDocument.getElementById(tabId(next))?.focus();
          onChange(next);
        }}>{t(item.labelKey)}</button>)}
    </div>
    {items.map(item => <div role="tabpanel" key={item.id} id={panelId(item.id)} aria-labelledby={tabId(item.id)}
      hidden={item.id !== active} tabIndex={0}>{item.panel}</div>)}
  </>;
}

export function SegmentedControl({ label, items, value, onChange }: ChoiceProps) {
  const name = useId();
  return <fieldset><legend>{label}</legend>{items.map(item => <label key={item.id}>
    <input type="radio" name={name} value={item.id} checked={item.id === value} disabled={item.disabled}
      onChange={() => onChange(item.id)} />{t(item.labelKey)}
  </label>)}</fieldset>;
}
