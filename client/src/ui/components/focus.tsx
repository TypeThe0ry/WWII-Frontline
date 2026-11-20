import type { ComponentChildren } from 'preact';
import { useId, useLayoutEffect, useRef } from 'preact/hooks';
import { t } from '../i18n';

// Positive tabindex precedes normal document order. Re-query on Tab because a form
// may have added, hidden, or disabled controls since the trap opened.
function tabStops(root: HTMLElement): HTMLElement[] {
  const candidates = root.querySelectorAll<HTMLElement>('a[href], button, input, textarea, select, summary, [contenteditable="true"], [tabindex]');
  const visible = Array.from(candidates).filter(el => !el.matches(':disabled, [type="hidden"]') && !el.closest('[inert], [hidden]')
    && (el.tabIndex >= 0 || (el.isContentEditable && !el.hasAttribute('tabindex'))) && el.getClientRects().length
    && getComputedStyle(el).visibility === 'visible');
  const stops = visible.filter(el => {
    if (!(el instanceof HTMLInputElement) || el.type !== 'radio' || !el.name) return true;
    const group = visible.filter((r): r is HTMLInputElement => r instanceof HTMLInputElement && r.type === 'radio' && r.name === el.name && r.form === el.form);
    return (group.find(r => r.checked) ?? group[0]) === el;
  });
  return stops.sort((a, b) => (a.tabIndex > 0 ? a.tabIndex : Infinity) - (b.tabIndex > 0 ? b.tabIndex : Infinity));
}

export function wrapTarget<T>(stops: readonly T[], current: T | null, backwards: boolean, fallback: T): T | undefined {
  if (!stops.length) return fallback;
  if (current === null || !stops.includes(current)) return backwards ? stops.at(-1) : stops[0];
  if (backwards && current === stops[0]) return stops.at(-1);
  if (!backwards && current === stops.at(-1)) return stops[0];
}

const activeTraps = new WeakMap<Document, HTMLElement[]>();

function activate(root: HTMLElement, returnFocus: boolean) {
  const doc = root.ownerDocument;
  const previous = doc.activeElement;
  const stack = activeTraps.get(doc) ?? [];
  activeTraps.set(doc, stack);
  const child = stack.findIndex(el => root.contains(el));
  stack.splice(child < 0 ? stack.length : child, 0, root);
  const top = () => stack.at(-1) === root;
  const focusFirst = () => (tabStops(root)[0] ?? root).focus();
  if (root instanceof HTMLDialogElement) root.showModal();
  if (top() && !root.contains(doc.activeElement)) focusFirst();
  const onFocus = (e: FocusEvent) => {
    if (top() && e.target instanceof Node && !root.contains(e.target)) focusFirst();
  };
  const onKey = (e: KeyboardEvent) => {
    if (!top() || e.key !== 'Tab' || e.defaultPrevented || e.altKey || e.ctrlKey || e.metaKey) return;
    const next = wrapTarget<HTMLElement>(tabStops(root), doc.activeElement instanceof HTMLElement ? doc.activeElement : null, e.shiftKey, root);
    if (next) { e.preventDefault(); next.focus(); }
  };
  doc.addEventListener('focusin', onFocus);
  doc.addEventListener('keydown', onKey);
  return () => {
    const wasTop = top();
    doc.removeEventListener('focusin', onFocus);
    doc.removeEventListener('keydown', onKey);
    stack.splice(stack.indexOf(root), 1);
    if (root instanceof HTMLDialogElement) root.close();
    if (wasTop && returnFocus && previous instanceof HTMLElement && previous.isConnected) previous.focus();
  };
}

export function FocusTrap({ active, returnFocus = true, children }: { active: boolean; returnFocus?: boolean; children?: ComponentChildren }) {
  const ref = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => { if (active && ref.current) return activate(ref.current, returnFocus); }, [active, returnFocus]);
  return <div ref={ref} tabIndex={-1}>{children}</div>;
}

export type Severity = 'info' | 'ok' | 'warn' | 'error';
export interface ActionSpec { labelKey: string; onClick(): void; disabled?: boolean }
export interface ModalProps {
  titleKey: string;
  children?: ComponentChildren;
  actions?: readonly ActionSpec[];
  onClose(): void;
  dismissible?: boolean;
  severity?: Severity;
}

export function Modal({ titleKey, onClose, children, actions = [], dismissible = true, severity }: ModalProps) {
  const ref = useRef<HTMLDialogElement>(null);
  const title = useId();
  useLayoutEffect(() => { if (ref.current) return activate(ref.current, true); }, []);
  return <dialog ref={ref} aria-labelledby={title} aria-modal="true" data-severity={severity}
    onCancel={e => { e.preventDefault(); if (dismissible) onClose(); }}>
    <h2 id={title}>{t(titleKey)}</h2>
    {children}
    {actions.map(action => <button key={action.labelKey} type="button" disabled={action.disabled}
      onClick={action.onClick}>{t(action.labelKey)}</button>)}
  </dialog>;
}
