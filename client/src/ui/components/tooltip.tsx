import { cloneElement, type VNode } from 'preact';
import { useLayoutEffect, useId, useRef, useState } from 'preact/hooks';
import { t, type Params } from '../i18n';

export interface TooltipProps {
  contentKey: string;
  params?: Params;
  placement?: 'top' | 'bottom' | 'left' | 'right';
  delayMs?: number;
  children: VNode<{ 'aria-describedby'?: string }>;
}

export function Tooltip({ contentKey, params, placement, delayMs = 0, children }: TooltipProps) {
  const id = useId();
  const ref = useRef<HTMLSpanElement>(null);
  const [hover, setHover] = useState(false);
  const [focus, setFocus] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  const [ready, setReady] = useState(false);
  const engaged = hover || focus;
  useLayoutEffect(() => {
    if (!engaged) { setDismissed(false); return; }
    const doc = ref.current!.ownerDocument;
    const hide = (e: KeyboardEvent) => {
      const modal = e.target instanceof Element && e.target.closest('dialog:modal');
      if (modal && !modal.contains(ref.current)) return;
      if (e.key === 'Escape' && !dismissed) { e.preventDefault(); setDismissed(true); }
    };
    doc.addEventListener('keydown', hide);
    const timer = setTimeout(() => setReady(true), Math.max(0, delayMs));
    return () => { clearTimeout(timer); setReady(false); doc.removeEventListener('keydown', hide); };
  }, [engaged, dismissed, delayMs]);
  const description = [children.props['aria-describedby'], id].filter(Boolean).join(' ');
  return <span ref={ref} onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}
    onFocusIn={() => setFocus(true)} onFocusOut={() => setFocus(false)}>
    {cloneElement(children, { 'aria-describedby': description })}
    <span id={id} role="tooltip" data-placement={placement} hidden={!engaged || dismissed || (delayMs > 0 && !ready)}>{t(contentKey, params)}</span>
  </span>;
}
