import { createContext, type ComponentChildren, type JSX } from 'preact';
import { useContext, useId } from 'preact/hooks';
import { DEFAULT_BINDINGS, type ActionId, type BindingMap } from '../../input/bindings';
import { t, type Params } from '../i18n';
import type { Severity } from './focus';
import { Tooltip } from './tooltip';

export type Tone = 'default' | 'friend' | 'enemy' | 'neutral' | 'unknown' | 'squad' | 'cmd' | 'accent' | 'ok' | 'warn' | 'danger';

export function Panel({ title, cornerCut, ticks, tone, children }: {
  title?: string; cornerCut?: 'sm' | 'md' | 'lg'; ticks?: boolean; tone?: Tone; children?: ComponentChildren;
}) {
  const id = useId();
  return <section data-corner-cut={cornerCut} data-ticks={ticks} data-tone={tone} aria-labelledby={title ? id : undefined}>
    {title && <h2 id={id}>{title}</h2>}{children}
  </section>;
}

export const KeyBindingsContext = createContext<Readonly<BindingMap>>(DEFAULT_BINDINGS);

export function KeyCap({ action, size }: { action: ActionId; size?: 'sm' | 'md' }) {
  const binding = useContext(KeyBindingsContext)[action];
  const code = binding.primary ?? binding.secondary;
  return code ? <kbd data-action={action} data-size={size}>{code.replace(/^(?:Key(?=[A-Z]$)|Digit(?=\d$))/, '')}</kbd> : null;
}

export interface ButtonProps extends Omit<JSX.IntrinsicElements['button'], 'size'> {
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost'; size?: 'sm' | 'md' | 'lg'; reason?: string; hotkey?: ActionId;
}

export function Button({ variant, size, reason, hotkey, disabled, onClick, type = 'button', children, ...attrs }: ButtonProps) {
  const explain = Boolean(disabled && reason);
  const button = <button {...attrs} type={type} data-variant={variant} data-size={size}
    disabled={disabled && !explain} aria-disabled={disabled || attrs['aria-disabled']}
    onClick={e => {
      if (disabled) { e.preventDefault(); e.stopPropagation(); return; }
      onClick?.(e);
    }}>{children}{hotkey && <KeyCap action={hotkey} />}</button>;
  return explain ? <Tooltip contentKey="common.disabledReason" params={{ reason: reason! }}>{button}</Tooltip> : button;
}

export function Icon({ id, size, tone, label }: { id: string; size?: 'S' | 'M' | 'L' | 'XL'; tone?: Tone; label?: string }) {
  return <svg data-size={size} data-tone={tone} focusable="false" role={label ? 'img' : undefined}
    aria-label={label} aria-hidden={label ? undefined : true}><use href={`#${id}`} /></svg>;
}

export function Frame({ affiliation, squad, glyph, level, status, label }: {
  affiliation: 'friend' | 'enemy' | 'neutral' | 'unknown'; squad?: boolean; glyph: string; level?: 0 | 1 | 2;
  status?: 'ok' | 'down' | 'dead'; label?: string;
}) {
  return <span data-affiliation={affiliation} data-squad={squad} data-level={level} data-status={status}>
    <Icon id={glyph} label={label} />
  </span>;
}

export interface ProgressProps { value: number; max: number; label: string; tone?: Tone; showText?: boolean; indeterminate?: boolean }
function Progress({ kind, value, max, label, tone, showText, indeterminate }: ProgressProps & { kind: 'ring' | 'bar' }) {
  const bound = max > 0 ? max : 1;
  const current = Math.min(bound, Math.max(0, value));
  return <span data-progress={kind} data-tone={tone}>
    <progress aria-label={label} value={indeterminate ? undefined : current} max={bound} />
    {showText && !indeterminate && <span aria-hidden="true">{current}/{bound}</span>}
  </span>;
}
export function ProgressRing(props: ProgressProps) { return <Progress {...props} kind="ring" />; }
export function ProgressBar(props: ProgressProps) { return <Progress {...props} kind="bar" />; }

export function Gauge({ value, min, max, ticks, tone, label }: {
  value: number; min: number; max: number; ticks: number; tone?: Tone; label: string;
}) {
  return <meter aria-label={label} min={min} max={max} value={value} data-ticks={ticks} data-tone={tone} />;
}

export interface Notice {
  id: string; kind: 'toast' | 'banner' | 'order' | 'tutorial' | 'dialog'; severity: Severity;
  i18nKey: string; params?: Params; ttlMs?: number; dedupeKey?: string; sound?: string;
}
function NoticeRegion({ notice, kind }: { notice: Notice; kind: 'toast' | 'banner' }) {
  return <div data-notice={kind} data-severity={notice.severity} role={notice.severity === 'error' ? 'alert' : 'status'}>
    {t(notice.i18nKey, notice.params)}
  </div>;
}
export function Toast({ notice }: { notice: Notice }) { return <NoticeRegion kind="toast" notice={notice} />; }
export function Banner({ notice }: { notice: Notice }) { return <NoticeRegion kind="banner" notice={notice} />; }

export function Skeleton({ shape, count = 1 }: { shape: 'row' | 'card' | 'text'; count?: number }) {
  return <div aria-hidden="true">{Array.from({ length: count }, (_, i) => <span key={i} data-shape={shape} />)}</div>;
}
